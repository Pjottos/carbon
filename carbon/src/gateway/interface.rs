use super::{
    message::{MessageBuf, MessageError, Write},
    registry::{GlobalObjectId, ObjectRegistry},
};

use std::{collections::VecDeque, os::unix::io::RawFd};

pub trait Interface {
    fn name(&self) -> &'static str;
    fn version(&self) -> u32;
    fn dispatch(
        &mut self,
        opcode: u16,
        args: &[u32],
        state: &mut DispatchState,
    ) -> Result<(), MessageError>;
}

pub struct DispatchState<'a> {
    pub fds: &'a mut VecDeque<RawFd>,
    pub send_buf: &'a mut MessageBuf<Write>,
    pub registry: &'a mut ObjectRegistry,
    pub objects: &'a mut Vec<Option<GlobalObjectId>>,
}
