use nix::{
    errno::Errno,
    sys::socket::{recvmsg, ControlMessageOwned, MsgFlags},
    unistd::close,
};

use std::{
    collections::VecDeque,
    io::{self, IoSlice, IoSliceMut},
    marker::PhantomData,
    os::unix::prelude::*,
};

const MAX_FDS_OUT: usize = 28;

/// Error returned when an invalid message was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageError {
    TooLarge,
    BadFormat,
    InvalidObject,
}

impl From<MessageError> for io::Error {
    fn from(e: MessageError) -> Self {
        let text = match e {
            MessageError::TooLarge => "Wayland message did not fit in buffer",
            MessageError::BadFormat => "Wayland message had incorrect format",
            MessageError::InvalidObject => "Wayland message had invalid object id",
        };
        io::Error::new(io::ErrorKind::InvalidData, text)
    }
}

pub struct MessageStream {
    stream_fd: RawFd,
    receive_buf: MessageBuf<Read>,
}

impl Drop for MessageStream {
    fn drop(&mut self) {
        let _ = close(self.stream_fd);
    }
}

impl MessageStream {
    pub fn new(stream_fd: RawFd) -> Self {
        Self {
            stream_fd,
            receive_buf: MessageBuf::new(),
        }
    }

    pub fn receive<D>(&mut self, mut dispatcher: D) -> io::Result<usize>
    where
        D: FnMut(u32, u16, &[u32], &mut VecDeque<RawFd>) -> Result<(), MessageError>,
    {
        let mut count = 0;
        let mut cmsg_buf = nix::cmsg_space!([RawFd; MAX_FDS_OUT]);
        loop {
            match recvmsg::<()>(
                self.stream_fd,
                &mut [self.receive_buf.io_slice_mut()],
                Some(&mut cmsg_buf),
                MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_NOSIGNAL,
            ) {
                Ok(msg) => {
                    if msg.bytes == 0 {
                        // Stream is closed, assume that there are no whole messages
                        // still in the buffer since they should have been processed
                        // in the last invocation.
                        return Ok(0);
                    }

                    self.receive_buf.grow(msg.bytes);
                    for cmsg in msg.cmsgs() {
                        if let ControlMessageOwned::ScmRights(fds) = cmsg {
                            self.receive_buf.fds.extend(fds.into_iter());
                        }
                    }
                }
                Err(e) if e == Errno::EINTR => {
                    // Should retry
                    continue;
                }
                Err(e) => return Err(e.into()),
            }

            let should_read = self.receive_buf.is_full();
            count += self.receive_buf.deserialize_messages(&mut dispatcher)?;
            if count == 0 {
                return if should_read {
                    Err(MessageError::TooLarge.into())
                } else {
                    Err(io::ErrorKind::WouldBlock.into())
                };
            }
            if should_read {
                continue;
            }
            return Ok(count);
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
    fn io_slice_mut(&mut self) -> IoSliceMut {
        let a = &mut bytemuck::cast_slice_mut(&mut self.buf)[self.len..];
        IoSliceMut::new(a)
    }

    /// `count` must be smaller than or equal to the length of the current `io_slice_mut`.
    #[inline]
    fn grow(&mut self, count: usize) {
        self.len += count;
    }

    fn deserialize_messages<D>(&mut self, dispatcher: &mut D) -> Result<usize, MessageError>
    where
        D: FnMut(u32, u16, &[u32], &mut VecDeque<RawFd>) -> Result<(), MessageError>,
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
