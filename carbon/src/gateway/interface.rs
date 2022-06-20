use super::{
    message::{MessageBuf, MessageError, Write},
    registry::{GlobalObjectId, ObjectRegistry},
};

use std::{collections::VecDeque, os::unix::io::RawFd};

pub enum Interface {
    WlDisplay,
}

pub struct DispatchState<'a> {
    pub fds: &'a mut VecDeque<RawFd>,
    pub send_buf: &'a mut MessageBuf<Write>,
    pub registry: &'a mut ObjectRegistry,
    pub objects: &'a mut Vec<Option<GlobalObjectId>>,
}
