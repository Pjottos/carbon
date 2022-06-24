use crate::{
    backend::Backend,
    gateway::{
        client::{Client, Clients},
        message::{FdSource, MessageError, MessageStream},
        registry::ObjectRegistry,
    },
    input::{InputSink, InputState},
    protocol::DispatchState,
};

use nix::{
    errno::Errno,
    fcntl::{flock, FlockArg},
    sys::{epoll::*, socket::*},
    unistd::close,
};
use num_enum::{IntoPrimitive, TryFromPrimitive, TryFromPrimitiveError};

use std::{env, fs, io, os::unix::prelude::*, path::PathBuf};

mod client;
pub mod message;
pub mod registry;

pub struct Gateway<B: Backend> {
    _lock_file: fs::File,
    listener_fd: RawFd,
    epoll_fd: RawFd,
    clients: Clients,
    registry: ObjectRegistry,
    input_state: InputState,
    backend: B,
}

impl<B: Backend> Drop for Gateway<B> {
    fn drop(&mut self) {
        let _ = close(self.listener_fd);
        let _ = close(self.epoll_fd);
    }
}

impl<B: Backend> Gateway<B> {
    pub fn new(backend: B) -> Self {
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
            .expect("Failed to add socket fd to epoll");
            let mut new_input_event = EpollEvent::new(
                EpollFlags::EPOLLIN | EpollFlags::EPOLLET,
                EpollToken {
                    kind: EpollTokenKind::NewInput,
                    id: 0,
                }
                .into(),
            );
            epoll_ctl(
                epoll_fd,
                EpollOp::EpollCtlAdd,
                backend.input_fd(),
                Some(&mut new_input_event),
            )
            .expect("Failed to add backend input fd to epoll");

            return Self {
                _lock_file: lock_file,
                listener_fd,
                epoll_fd,
                clients: Clients::new(),
                registry: ObjectRegistry::new(),
                backend,
                input_state: InputState::new(),
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

                        self.handle_epoll(token, event.events());
                    }
                }
                Err(e) => panic!("Error waiting for epoll event: {}", e),
            }
        }
    }

    fn handle_epoll(&mut self, token: EpollToken, events: EpollFlags) {
        use EpollTokenKind::*;

        match token.kind {
            NewConnection => {
                let stream_fd = match accept4(
                    self.listener_fd,
                    SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
                ) {
                    Ok(fd) => fd,
                    Err(Errno::EWOULDBLOCK) => return,
                    Err(e) => {
                        log::error!("Failed to accept socket connection: {}", e);
                        return;
                    }
                };

                let client_id = self.clients.next_id();
                let mut client_data_event = EpollEvent::new(
                    EpollFlags::EPOLLIN | EpollFlags::EPOLLOUT | EpollFlags::EPOLLET,
                    EpollToken {
                        kind: EpollTokenKind::ClientData,
                        id: client_id,
                    }
                    .into(),
                );
                if let Err(e) = epoll_ctl(
                    self.epoll_fd,
                    EpollOp::EpollCtlAdd,
                    stream_fd,
                    Some(&mut client_data_event),
                ) {
                    log::error!("Failed to register stream with epoll: {}", e);
                    let _ = close(stream_fd);
                    return;
                }

                let client = Client::new(MessageStream::new(stream_fd), self.registry.display_id());
                self.clients.insert_or_push(client_id, client);
            }
            ClientData => {
                let (stream, objects) = match self.clients.get_mut(token.id) {
                    Some(client) => client.stream_and_objects_mut(),
                    None => {
                        log::error!("Received ready event for non-existing client");
                        return;
                    }
                };

                if events.contains(EpollFlags::EPOLLIN) {
                    let dispatcher =
                        |object_id, opcode, args: &_, fds: FdSource<'_>, send_buf: &mut _| {
                            let global_id =
                                objects.get(object_id).ok_or(MessageError::InvalidObject)?;

                            if let Some(mut object) = self.registry.take(global_id) {
                                let mut state = DispatchState {
                                    fds,
                                    send_buf,
                                    registry: &mut self.registry,
                                    objects,
                                };
                                let res = object.dispatch(opcode, args, &mut state);
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
                            self.clients.delete(token.id);
                            return;
                        }
                        Ok(count) => {
                            log::debug!("Processed {} requests", count);
                        }
                        Err(MessageError::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => (),
                        Err(e) => {
                            log::error!("Error while receiving message: {}", e);
                            log::error!("Dropping this client");
                            self.clients.delete(token.id);
                            return;
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
                            self.clients.delete(token.id);
                        }
                    }
                }
            }
            NewInput => {
                let mut sink = InputSink {
                    state: &mut self.input_state,
                    registry: &mut self.registry,
                };
                match self.backend.drain_input(&mut sink) {
                    Ok(_) => (),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
                    Err(e) => {
                        log::error!("Backend failed to drain input: {}", e);
                    }
                }
            }
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
    NewInput,
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
