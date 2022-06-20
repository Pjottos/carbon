use self::{
    client::Client,
    interface::DispatchState,
    message::{MessageError, MessageStream},
    registry::ObjectRegistry,
};

use nix::{
    fcntl::{flock, FlockArg},
    sys::{epoll::*, socket::*},
    unistd::close,
};
use num_enum::{IntoPrimitive, TryFromPrimitive, TryFromPrimitiveError};

use std::{env, fs, intrinsics::discriminant_value, io, os::unix::prelude::*, path::PathBuf};

mod client;
mod interface;
mod message;
mod registry;

pub struct Gateway {
    _lock_file: fs::File,
    listener_fd: RawFd,
    epoll_fd: RawFd,
    clients: Vec<Option<Client>>,
    registry: ObjectRegistry,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = close(self.listener_fd);
    }
}

impl Gateway {
    pub fn new() -> Self {
        let mut path: PathBuf = env::var("XDG_RUNTIME_DIR")
            .ok()
            .filter(|dir| dir.starts_with('/'))
            .expect("XDG_RUNTIME_DIR not set or invalid")
            .into();

        for n in 0..32 {
            if n != 0 {
                path.pop();
            }
            path.push(format!("wayland-{}.lock", n));

            let lock_file = match fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .mode(0o660)
                .open(&path)
            {
                Ok(f) => f,
                Err(e) => {
                    log::warn!(
                        "Failed to open socket lock file at {}: {}",
                        path.to_string_lossy(),
                        e
                    );
                    continue;
                }
            };

            if let Err(e) = flock(lock_file.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
                log::warn!(
                    "Failed to acquire socket lock at {}: {}",
                    path.to_string_lossy(),
                    e.desc()
                );
                continue;
            };

            path.set_extension("");
            if let Some(e) = fs::remove_file(&path)
                .err()
                .filter(|e| e.kind() != io::ErrorKind::NotFound)
            {
                log::warn!(
                    "Failed to remove existing socket at {}: {}",
                    path.to_string_lossy(),
                    e
                );
                continue;
            }

            let sock_addr = UnixAddr::new(&path).unwrap();
            let listener_fd = socket(
                AddressFamily::Unix,
                SockType::Stream,
                SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
                None,
            )
            .expect("Failed to create socket");
            bind(listener_fd, &sock_addr)
                .and_then(|_| listen(listener_fd, 256))
                .map_err(|e| {
                    let _ = close(listener_fd);
                    e
                })
                .expect("Failed to bind to socket");

            log::info!("Listening at {}", path.to_string_lossy());

            let epoll_fd = epoll_create1(EpollCreateFlags::EPOLL_CLOEXEC)
                .expect("Failed to create epoll instance");
            let mut new_connection_event = EpollEvent::new(
                EpollFlags::EPOLLIN | EpollFlags::EPOLLET,
                EpollToken {
                    kind: EpollTokenKind::NewConnection,
                    id: 0,
                }
                .into(),
            );
            epoll_ctl(
                epoll_fd,
                EpollOp::EpollCtlAdd,
                listener_fd,
                Some(&mut new_connection_event),
            )
            .expect("Failed to add socket to epoll");

            return Self {
                _lock_file: lock_file,
                listener_fd,
                epoll_fd,
                clients: vec![],
                registry: ObjectRegistry::new(),
            };
        }

        panic!("Could not find a socket to bind to");
    }

    pub fn run(&mut self) {
        let mut events = [EpollEvent::empty(); 256];

        loop {
            match epoll_wait(self.epoll_fd, &mut events, -1) {
                Ok(count) => {
                    for event in &events[..count] {
                        let token: EpollToken = event
                            .data()
                            .try_into()
                            .expect("Received invalid event data");

                        match self.handle_epoll(token, event.events()) {
                            Ok(_) => (),
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
                            Err(e) => panic!("Unhandled IO error: {}", e),
                        }
                    }
                }
                Err(e) => panic!("Error waiting for epoll event: {}", e),
            }
        }
    }

    fn handle_epoll(&mut self, token: EpollToken, events: EpollFlags) -> io::Result<()> {
        use EpollTokenKind::*;

        match token.kind {
            NewConnection => {
                let stream_fd = accept4(
                    self.listener_fd,
                    SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
                )?;

                let client_id = self.next_client_id();
                let mut client_data_event = EpollEvent::new(
                    EpollFlags::EPOLLIN | EpollFlags::EPOLLOUT | EpollFlags::EPOLLET,
                    EpollToken {
                        kind: EpollTokenKind::ClientData,
                        id: client_id,
                    }
                    .into(),
                );
                epoll_ctl(
                    self.epoll_fd,
                    EpollOp::EpollCtlAdd,
                    stream_fd,
                    Some(&mut client_data_event),
                )?;

                let client = Client::new(MessageStream::new(stream_fd), self.registry.display_id());
                match self.clients.get_mut(client_id as usize) {
                    Some(entry) => *entry = Some(client),
                    None => self.clients.push(Some(client)),
                }
            }
            ClientData => {
                let (stream, objects) = match self
                    .clients
                    .get_mut(token.id as usize)
                    .and_then(Option::as_mut)
                {
                    Some(client) => client.stream_and_objects_mut(),
                    None => {
                        log::warn!("Received ready event for non-existing client");
                        return Ok(());
                    }
                };

                if events.contains(EpollFlags::EPOLLIN) {
                    let dispatcher =
                        |object_id, opcode, args: &_, fds: &mut _, send_buf: &mut _| {
                            let global_id = objects
                                .get(object_id as usize)
                                .and_then(|id| *id)
                                .ok_or(MessageError::InvalidObject)?;

                            if let Some(object) = self.registry.take(global_id) {
                                let mut state = DispatchState {
                                    fds,
                                    send_buf,
                                    registry: &mut self.registry,
                                    objects,
                                };
                                let res = registry::INTERFACE_DISPATCH_TABLE
                                    .get(discriminant_value(&object) as usize)
                                    .and_then(|funcs| funcs.get(opcode as usize))
                                    .and_then(Option::as_ref)
                                    .ok_or(MessageError::InvalidOpcode)
                                    .and_then(|f| f(args, &mut state));
                                self.registry.restore(global_id, object);
                                res?;
                            } else {
                                // Can happen if object has been deleted but the client has not
                                // yet acknowledged it.
                                log::debug!("Attempt to dispatch request for deleted object");
                            }

                            Ok(())
                        };

                    match stream.receive(dispatcher) {
                        Ok(0) => {
                            log::debug!("Client disconnected");
                            self.delete_client(token.id);
                            return Ok(());
                        }
                        Ok(count) => {
                            log::debug!("Processed {} requests", count);
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
                        Err(e) => {
                            log::error!("Error while receiving message: {}", e);
                            log::error!("Dropping this client");
                            self.delete_client(token.id);
                            return Ok(());
                        }
                    }
                }
                if events.contains(EpollFlags::EPOLLOUT) {
                    match stream.flush() {
                        Ok(0) => (),
                        Ok(count) => {
                            log::debug!("Flushed {} bytes", count);
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
                        Err(e) => {
                            log::error!("Error while flushing messages: {}", e);
                            log::error!("Dropping this client");
                            self.delete_client(token.id);
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn next_client_id(&self) -> u32 {
        self.clients
            .iter()
            .take_while(|c| c.is_some())
            .count()
            .try_into()
            .expect("Too many clients")
    }

    fn delete_client(&mut self, id: u32) {
        if let Some(entry) = self.clients.get_mut(id as usize) {
            let _ = entry.take();
        }
    }
}

#[derive(Debug)]
struct EpollToken {
    kind: EpollTokenKind,
    id: u32,
}

#[derive(Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
enum EpollTokenKind {
    NewConnection,
    ClientData,
}

impl From<EpollToken> for u64 {
    fn from(token: EpollToken) -> Self {
        let kind: u32 = token.kind.into();
        u64::from(kind) | (u64::from(token.id) << 32)
    }
}

impl TryFrom<u64> for EpollToken {
    type Error = TryFromPrimitiveError<EpollTokenKind>;

    fn try_from(raw: u64) -> Result<Self, Self::Error> {
        let kind = (raw as u32).try_into()?;
        let id = (raw >> 32) as u32;

        Ok(Self { kind, id })
    }
}
