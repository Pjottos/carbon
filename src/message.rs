use atomic_refcell::AtomicRefCell;
use nix::{
    errno::Errno,
    libc,
    sys::socket::{recvmsg, ControlMessageOwned, MsgFlags},
};
use tokio::{io, net::UnixStream};

use std::{io::IoSliceMut, os::unix::prelude::*};

const MAX_FDS_OUT: usize = 28;

pub struct MessageStream {
    stream: UnixStream,
    receive_buf: AtomicRefCell<MessageBuf>,
}

impl MessageStream {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            receive_buf: AtomicRefCell::new(MessageBuf::new()),
        }
    }

    pub async fn receive(&self) -> io::Result<Option<()>> {
        loop {
            self.stream.readable().await?;

            let raw_fd = self.stream.as_raw_fd();
            let mut cmsg_buf = nix::cmsg_space!([RawFd; MAX_FDS_OUT]);
            let mut receive_buf = self.receive_buf.borrow_mut();
            let (mut vecs, vec_count) = receive_buf.io_vecs_mut();
            // Can't add directly to receive_buf because it has a live mutable borrow
            let mut new_fds = vec![];
            let res = self.stream.try_io(io::Interest::READABLE, || loop {
                match recvmsg::<()>(
                    raw_fd,
                    &mut vecs[..vec_count],
                    Some(&mut cmsg_buf),
                    MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_NOSIGNAL,
                ) {
                    Ok(msg) => {
                        for cmsg in msg.cmsgs() {
                            if let ControlMessageOwned::ScmRights(mut fds) = cmsg {
                                new_fds.append(&mut fds);
                            }
                        }

                        break Ok(msg.bytes);
                    }
                    Err(e) if e == Errno::EWOULDBLOCK || e == Errno::EAGAIN => {
                        break Err(io::ErrorKind::WouldBlock.into());
                    }
                    Err(e) if e == Errno::EINTR => {
                        continue;
                    }
                    Err(e) => break Err(e.into()),
                }
            });

            match res {
                Ok(count) => {
                    if count == 0 {
                        // Stream is closed, one last try to get a message
                        return Ok(receive_buf.try_deserialize());
                    }

                    receive_buf.grow(count)?;
                    receive_buf.fds.append(&mut new_fds);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // .readable() gave a false positive, try again
                    continue;
                }
                Err(e) => return Err(e),
            }

            let deserialized = receive_buf.try_deserialize();
            if deserialized.is_some() {
                return Ok(deserialized);
            }

            // Not enough data yet, continue reading
        }
    }
}

struct MessageBuf {
    buf: [u8; Self::BUF_SIZE],
    head: usize,
    tail: usize,
    fds: Vec<RawFd>,
}

impl MessageBuf {
    const BUF_SIZE: usize = 4096;

    fn new() -> Self {
        Self {
            buf: [0; Self::BUF_SIZE],
            head: 0,
            tail: 0,
            fds: vec![],
        }
    }

    fn io_vecs_mut(&mut self) -> ([IoSliceMut; 2], usize) {
        let (a, b) = self.buf.split_at_mut(self.head);

        if self.head < self.tail {
            let a = &mut [];
            let b = &mut b[..self.tail];
            ([IoSliceMut::new(b), IoSliceMut::new(a)], 1)
        } else if self.tail == 0 {
            let a = &mut [];
            ([IoSliceMut::new(b), IoSliceMut::new(a)], 1)
        } else {
            let a = &mut a[..self.tail];
            ([IoSliceMut::new(b), IoSliceMut::new(a)], 2)
        }
    }

    fn grow(&mut self, count: usize) -> io::Result<()> {
        let new_head = self.head + count;
        if new_head - self.tail >= Self::BUF_SIZE {
            Err(io::Error::from_raw_os_error(libc::EOVERFLOW))
        } else {
            self.head = new_head % Self::BUF_SIZE;
            Ok(())
        }
    }

    fn try_deserialize(&mut self) -> Option<()> {
        log::info!("{:02x?}", &self.buf[..self.head]);
        if self.head > 0 {
            self.head = 0;
            Some(())
        } else {
            None
        }
    }
}
