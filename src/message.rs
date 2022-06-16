use atomic_refcell::AtomicRefCell;
use byteorder::{NativeEndian, ReadBytesExt};
use nix::{
    errno::Errno,
    sys::socket::{recvmsg, ControlMessageOwned, MsgFlags},
};
use tokio::{io, net::UnixStream};

use std::{io::IoSliceMut, os::unix::prelude::*};

const MAX_FDS_OUT: usize = 28;

/// Error returned when an invalid message was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageError {
    TooLarge,
    BadFormat,
}

impl From<MessageError> for io::Error {
    fn from(e: MessageError) -> Self {
        let text = match e {
            MessageError::TooLarge => "Wayland message did not fit in buffer",
            MessageError::BadFormat => "Wayland message had incorrect format",
        };
        io::Error::new(io::ErrorKind::InvalidData, text)
    }
}

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
            {
                let mut receive_buf = self.receive_buf.borrow_mut();
                let deserialized = receive_buf.try_deserialize();
                if deserialized.is_some() {
                    return deserialized.transpose().map_err(|e| e.into());
                }
            }

            self.stream.readable().await?;

            let raw_fd = self.stream.as_raw_fd();
            let mut cmsg_buf = nix::cmsg_space!([RawFd; MAX_FDS_OUT]);
            let mut receive_buf = self.receive_buf.borrow_mut();
            let (mut vecs, vec_count) = receive_buf.io_vecs_mut()?;
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
                        return receive_buf
                            .try_deserialize()
                            .transpose()
                            .map_err(|e| e.into());
                    }

                    receive_buf.grow(count);
                    receive_buf.fds.append(&mut new_fds);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // .readable() gave a false positive, try again
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

struct MessageBuf {
    buf: [u8; Self::BUF_SIZE],
    head: usize,
    tail: usize,
    len: usize,
    fds: Vec<RawFd>,
}

impl MessageBuf {
    const BUF_SIZE: usize = 4096;

    #[inline]
    fn new() -> Self {
        Self {
            buf: [0; Self::BUF_SIZE],
            head: 0,
            tail: 0,
            len: 0,
            fds: vec![],
        }
    }

    fn is_full(&self) -> bool {
        self.len == Self::BUF_SIZE
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn io_vecs_mut(&mut self) -> Result<([IoSliceMut; 2], usize), MessageError> {
        if self.is_full() {
            Err(MessageError::TooLarge)
        } else if self.is_empty() {
            self.head = 0;
            self.tail = 0;
            Ok((
                [IoSliceMut::new(&mut self.buf), IoSliceMut::new(&mut [])],
                1,
            ))
        } else if self.tail == 0 {
            let a = &mut self.buf[self.head..];
            Ok(([IoSliceMut::new(a), IoSliceMut::new(&mut [])], 1))
        } else if self.head < self.tail {
            let a = &mut self.buf[self.head..self.tail];
            Ok(([IoSliceMut::new(a), IoSliceMut::new(&mut [])], 1))
        } else {
            let (b, a) = self.buf.split_at_mut(self.head);
            let b = &mut b[..self.tail];
            Ok(([IoSliceMut::new(a), IoSliceMut::new(b)], 2))
        }
    }

    fn grow(&mut self, count: usize) {
        assert!(self.len + count <= Self::BUF_SIZE);
        self.head = (self.head + count) % Self::BUF_SIZE;
        self.len += count;
    }

    fn shrink(&mut self, count: usize) {
        self.tail = (self.tail + count) % Self::BUF_SIZE;
        self.len = self.len.saturating_sub(count);
    }

    fn try_deserialize(&mut self) -> Option<Result<(), MessageError>> {
        let mut reader = RingBufReader {
            buf: &self.buf,
            tail: self.tail,
            count: self.len,
        };

        let object_id = reader.read_u32::<NativeEndian>().ok()?;
        let header = reader.read_u32::<NativeEndian>().ok()?;
        let msg_size = (header >> 16) as usize;
        let opcode = header & 0xFFFF;

        if msg_size < 8 || msg_size % 4 != 0 {
            return Some(Err(MessageError::BadFormat));
        }

        log::debug!("object_id : {}", object_id);
        log::debug!("opcode    : {}", opcode);
        log::debug!("msg_size  : {}", msg_size);

        if self.len >= msg_size {
            // TODO: deserialize message
            for _ in (8..msg_size).step_by(4) {
                log::debug!(
                    "payload   : {:08x}",
                    reader.read_u32::<NativeEndian>().ok()?
                );
            }

            self.shrink(msg_size);
            self.fds.clear();

            Some(Ok(()))
        } else {
            None
        }
    }
}

struct RingBufReader<'a> {
    buf: &'a [u8; MessageBuf::BUF_SIZE],
    tail: usize,
    count: usize,
}

impl<'a> std::io::Read for RingBufReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.count == 0 {
            return Ok(0);
        }

        let (a, b) = self.buf.split_at(self.tail);
        let count = self.count.min(buf.len());

        if count <= b.len() {
            buf[..count].copy_from_slice(&b[..count]);
        } else {
            buf[..b.len()].copy_from_slice(b);
            buf[b.len()..count].copy_from_slice(&a[..count - b.len()]);
        }

        self.tail = (self.tail + count) % MessageBuf::BUF_SIZE;
        self.count -= count;

        Ok(count)
    }
}
