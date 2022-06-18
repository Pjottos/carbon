use nix::{
    errno::Errno,
    sys::socket::{recvmsg, ControlMessageOwned, MsgFlags},
};
use tokio::{io, net::UnixStream};

use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{IoSlice, IoSliceMut},
    marker::PhantomData,
    os::unix::prelude::*,
};

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
    receive_buf: RefCell<MessageBuf<Read>>,
}

impl MessageStream {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            receive_buf: RefCell::new(MessageBuf::new()),
        }
    }

    pub async fn receive<D>(&self, dispatcher: D) -> io::Result<usize>
    where
        D: Fn(u32, u16, &[u32], &mut VecDeque<RawFd>) -> Result<(), MessageError>,
    {
        loop {
            {
                let mut receive_buf = self.receive_buf.borrow_mut();
                let count = receive_buf.deserialize_messages(&dispatcher)?;
                if count != 0 {
                    return Ok(count);
                }
            }

            self.stream.readable().await?;

            let raw_fd = self.stream.as_raw_fd();
            let mut cmsg_buf = nix::cmsg_space!([RawFd; MAX_FDS_OUT]);
            let mut receive_buf = self.receive_buf.borrow_mut();
            let io_slices = &mut [receive_buf.io_slice_mut()?];
            // Can't add directly to receive_buf because it has a live mutable borrow
            let mut new_fds = vec![];
            let res = self.stream.try_io(io::Interest::READABLE, || loop {
                match recvmsg::<()>(
                    raw_fd,
                    io_slices,
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
                        // Stream is closed, we did not process any messages before
                        // reading from the stream and there is no new data so it
                        // is not possible for new messages to have arrived.
                        return Ok(0);
                    }

                    receive_buf.grow(count);
                    receive_buf.fds.extend(new_fds.into_iter());
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

const BUF_SIZE: usize = 4096;

struct Read;
struct Write;
struct MessageBuf<I> {
    buf: [u32; BUF_SIZE / 4],
    len: usize,
    fds: VecDeque<RawFd>,
    _phantom: PhantomData<I>,
}

impl<I> MessageBuf<I> {
    #[inline]
    fn new() -> Self {
        Self {
            buf: [0; BUF_SIZE / 4],
            len: 0,
            fds: VecDeque::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn is_full(&self) -> bool {
        self.len == BUF_SIZE
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl MessageBuf<Read> {
    #[inline]
    fn io_slice_mut(&mut self) -> Result<IoSliceMut, MessageError> {
        if self.is_full() {
            Err(MessageError::TooLarge)
        } else {
            let a = &mut bytemuck::cast_slice_mut(&mut self.buf)[self.len..];
            Ok(IoSliceMut::new(a))
        }
    }

    /// `count` must be smaller than or equal to the length of the current `io_slice_mut`.
    #[inline]
    fn grow(&mut self, count: usize) {
        self.len += count;
    }

    fn deserialize_messages<D>(&mut self, dispatcher: &D) -> Result<usize, MessageError>
    where
        D: Fn(u32, u16, &[u32], &mut VecDeque<RawFd>) -> Result<(), MessageError>,
    {
        let mut idx = 0;
        let mut msg_count = 0;

        // While we have enough bytes for a message header
        while self.len >= 8 {
            let object_id = self.buf[idx];
            let header = self.buf[idx + 1];
            let msg_size = (header >> 16) as usize;
            let opcode = header as u16;

            log::debug!("object_id : {}", object_id);
            log::debug!("opcode    : {}", opcode);
            log::debug!("msg_size  : {}", msg_size);

            if msg_size < 8 || msg_size % 4 != 0 {
                return Err(MessageError::BadFormat);
            }

            if self.len >= msg_size {
                let msg_end = idx + msg_size / 4;
                let payload = &self.buf[idx + 2..msg_end];
                for v in payload {
                    log::debug!("payload   : {:08x}", v,);
                }
                dispatcher(object_id, opcode, payload, &mut self.fds)?;
                msg_count += 1;

                self.len -= msg_size;
                idx = msg_end;
            } else {
                // We haven't received the full message yet
                break;
            }
        }

        if self.len > 0 && idx != 0 {
            // Copy remaining partial message to start of buffer.
            // This is not very likely to happen, and most messages are also quite small.
            self.buf.copy_within(idx..idx + (self.len + 3) / 4, 0);
        }

        Ok(msg_count)
    }
}

impl MessageBuf<Write> {
    #[inline]
    fn io_slice(&self) -> IoSlice {
        let a = &bytemuck::cast_slice(&self.buf)[..self.len];
        IoSlice::new(a)
    }

    #[inline]
    fn allocate(&mut self, chunk_count: usize) -> Option<&mut [u32]> {
        let new_len = self.len + chunk_count * 4;
        (new_len <= BUF_SIZE).then(|| {
            let res = &mut self.buf[self.len / 4..new_len / 4];
            self.len = new_len;
            res
        })
    }
}
