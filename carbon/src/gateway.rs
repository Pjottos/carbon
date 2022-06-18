use crate::message::MessageStream;

use nix::fcntl::{flock, FlockArg};
use tokio::{net::UnixListener, task};

use std::{env, fs, io, os::unix::prelude::*, path::PathBuf};

pub struct Gateway {
    _lock_file: fs::File,
    listener: UnixListener,
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

            let listener = UnixListener::bind(&path).expect("Failed to bind to socket");

            log::info!("Bound socket at {}", path.to_string_lossy());

            return Self {
                _lock_file: lock_file,
                listener,
            };
        }

        panic!("Could not find a socket to bind to");
    }

    pub async fn listen(&self) {
        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let stream = MessageStream::new(stream);
                    task::spawn_local(async move {
                        loop {
                            match stream.receive(|_, _, _, _| Ok(())).await {
                                Ok(count) if count != 0 => {
                                    log::debug!("Processed {} requests", count);
                                }
                                Ok(_) => {
                                    log::debug!("Client disconnected");
                                    break;
                                }
                                Err(e) => {
                                    log::error!("Error while receiving message: {}", e);
                                    log::error!("Dropping this client");
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => log::error!("Failed to accept socket connection: {}", e),
            }
        }
    }
}
