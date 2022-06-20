use crate::gateway::{
    message::{MessageBuf, MessageError, Write},
    registry::{GlobalObjectId, ObjectRegistry},
};

use std::{collections::VecDeque, intrinsics::discriminant_value, os::unix::io::RawFd};

mod generated;
pub use generated::Interface;

mod wayland;
pub use wayland::*;

impl Interface {
    #[inline]
    pub fn dispatch(
        &mut self,
        opcode: u16,
        args: &[u32],
        state: &mut DispatchState,
    ) -> Result<(), MessageError> {
        generated::INTERFACE_DISPATCH_TABLE
            .get(discriminant_value(self) as usize)
            .and_then(|funcs| funcs.get(opcode as usize))
            .and_then(Option::as_ref)
            .ok_or(MessageError::InvalidOpcode)
            .and_then(|f| f(args, &mut state))
    }
}

pub struct DispatchState<'a> {
    pub fds: &'a mut VecDeque<RawFd>,
    pub send_buf: &'a mut MessageBuf<Write>,
    pub registry: &'a mut ObjectRegistry,
    pub objects: &'a mut Vec<Option<GlobalObjectId>>,
}
