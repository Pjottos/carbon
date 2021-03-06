use crate::gateway::registry::ObjectId;

use nix::{
    errno::Errno,
    sys::socket::{recvmsg, sendmsg, ControlMessage, ControlMessageOwned, MsgFlags},
    unistd::close,
};

use std::{
    collections::VecDeque,
    io::{self, IoSlice, IoSliceMut},
    marker::PhantomData,
    os::unix::prelude::*,
};

const MAX_FDS_OUT: usize = 28;

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("out of memory")]
    OutOfMemory,
    #[error("request had bad wire format: {0}")]
    BadFormat(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("bad object id")]
    InvalidObject,
    #[error("bad request opcode")]
    InvalidOpcode,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub struct MessageStream {
    stream_fd: RawFd,
    receive_buf: MessageBuf<Read>,
    send_buf: MessageBuf<Write>,
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
            send_buf: MessageBuf::new(),
        }
    }

    #[inline]
    pub fn send_buf_mut(&mut self) -> &mut MessageBuf<Write> {
        &mut self.send_buf
    }

    pub fn receive<D>(&mut self, mut dispatcher: D) -> Result<usize, MessageError>
    where
        D: FnMut(
            ObjectId,
            u16,
            &[u32],
            FdSource,
            &mut MessageBuf<Write>,
        ) -> Result<(), MessageError>,
    {
        let mut count = 0;
        let mut cmsg_buf = nix::cmsg_space!([RawFd; MAX_FDS_OUT]);
        loop {
            if self.receive_buf.is_full() {
                return Err(MessageError::OutOfMemory);
            }

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
                        //
                        // Note that we may have actually processed some messages,
                        // but we must return 0 to communicate that the stream is closed
                        // since, probably, no additional wakeups will be issued.
                        return Ok(0);
                    }

                    self.receive_buf.grow(msg.bytes);
                    for cmsg in msg.cmsgs() {
                        if let ControlMessageOwned::ScmRights(fds) = cmsg {
                            self.receive_buf.fds.extend(fds.into_iter());
                        }
                    }
                }
                Err(Errno::EWOULDBLOCK) => break,
                Err(e) => return Err(io::Error::from(e).into()),
            }

            count += self
                .receive_buf
                .deserialize_messages(&mut dispatcher, &mut self.send_buf)?;
        }

        if count == 0 {
            Err(io::Error::from(io::ErrorKind::WouldBlock).into())
        } else {
            Ok(count)
        }
    }

    pub fn flush(&mut self) -> io::Result<usize> {
        let mut total_count = 0;
        let res = loop {
            if total_count == self.send_buf.len() {
                break Ok(total_count);
            }

            self.send_buf.fds.make_contiguous();
            let control_messages = [ControlMessage::ScmRights(self.send_buf.fds.as_slices().0)];
            let control_message_count = (!self.send_buf.fds.is_empty()) as usize;

            match sendmsg::<()>(
                self.stream_fd,
                &[self.send_buf.io_slice(total_count)],
                &control_messages[..control_message_count],
                MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_NOSIGNAL,
                None,
            ) {
                Ok(count) => {
                    total_count += count;
                    self.send_buf.fds.clear();
                }
                Err(Errno::EWOULDBLOCK) => {
                    break if total_count == 0 {
                        // Buffer length is not zero and we did not manage to write any bytes
                        // so return WouldBlock.
                        Err(io::Error::from(io::ErrorKind::WouldBlock))
                    } else {
                        Ok(total_count)
                    };
                }
                Err(e) => break Err(e.into()),
            }
        };

        // if total_count != 0 {
        //     log::debug!("Flushed buffer:");
        //     for chunk in &self.send_buf.buf[..total_count / 4] {
        //         let bytes = chunk.to_ne_bytes();
        //         log::debug!(
        //             "{:08x} {}",
        //             chunk.swap_bytes(),
        //             String::from_utf8_lossy(&bytes)
        //         );
        //     }
        // }

        self.send_buf.shrink(total_count);

        res
    }
}

pub struct FdSource<'a>(&'a mut VecDeque<RawFd>);

impl<'a> FdSource<'a> {
    pub fn pop(&mut self) -> Option<RawFd> {
        self.0.pop_front()
    }
}

const BUF_SIZE: usize = 4096;

struct Read;
pub struct Write;
pub struct MessageBuf<I> {
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
    fn len(&self) -> usize {
        self.len
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

    fn deserialize_messages<D>(
        &mut self,
        dispatcher: &mut D,
        send_buf: &mut MessageBuf<Write>,
    ) -> Result<usize, MessageError>
    where
        D: FnMut(
            ObjectId,
            u16,
            &[u32],
            FdSource,
            &mut MessageBuf<Write>,
        ) -> Result<(), MessageError>,
    {
        let mut idx = 0;
        let mut msg_count = 0;

        let res = loop {
            // While we have enough bytes for a message header
            if self.len < 8 {
                break Ok(msg_count);
            }

            let object_id = match ObjectId::new(self.buf[idx]) {
                Some(id) => id,
                None => break Err(MessageError::InvalidObject),
            };
            let header = self.buf[idx + 1];
            let msg_size = (header >> 16) as usize;
            let opcode = header as u16;

            if msg_size < 8 || msg_size % 4 != 0 {
                break Err(MessageError::BadFormat(
                    "message size < 8 or not a multiple of 4".to_owned(),
                ));
            }

            if self.len >= msg_size {
                let msg_end = idx + msg_size / 4;
                let payload = &self.buf[idx + 2..msg_end];

                // log::debug!("object : {:?}", object_id);
                // log::debug!("size   : {:?}", msg_size);
                // log::debug!("opcode : {:?}", opcode);
                // for chunk in payload {
                //     let bytes = chunk.to_ne_bytes();
                //     log::debug!(
                //         "payload: {:08x} {}",
                //         chunk.swap_bytes(),
                //         String::from_utf8_lossy(&bytes)
                //     );
                // }
                // log::debug!("------------------------------");

                if let Err(e) = dispatcher(
                    object_id,
                    opcode,
                    payload,
                    FdSource(&mut self.fds),
                    send_buf,
                ) {
                    break Err(e);
                }
                msg_count += 1;

                self.len -= msg_size;
                idx = msg_end;
            } else {
                // We haven't received the full message yet
                break Ok(msg_count);
            }
        };

        if self.len > 0 && idx != 0 {
            // Copy remaining partial message to start of buffer.
            // This is not very likely to happen, and most messages are also quite small.
            //
            // It's also possible a request failed in which case we should clean up the
            // buffer by removing the requests we processed.
            self.buf.copy_within(idx..idx + (self.len + 3) / 4, 0);
        }

        res
    }
}

impl MessageBuf<Write> {
    #[inline]
    fn io_slice(&self, offset: usize) -> IoSlice {
        let a = &bytemuck::cast_slice(&self.buf)[offset..self.len];
        IoSlice::new(a)
    }

    #[inline]
    pub fn allocate(&mut self, chunk_count: usize) -> Result<&mut [u32], MessageError> {
        let new_len = self.len + chunk_count * 4;
        (new_len <= BUF_SIZE)
            .then(|| {
                let res = &mut self.buf[self.len / 4..new_len / 4];
                self.len = new_len;
                res
            })
            .ok_or(MessageError::OutOfMemory)
    }

    #[inline]
    pub fn push_fd(&mut self, fd: RawFd) -> Result<(), MessageError> {
        if self.fds.len() < MAX_FDS_OUT {
            self.fds.push_back(fd);
            Ok(())
        } else {
            Err(MessageError::OutOfMemory)
        }
    }

    fn shrink(&mut self, count: usize) {
        if count < self.len {
            let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut self.buf);
            bytes.copy_within(count..self.len, 0);
        }
        self.len -= count;
    }
}
