use crate::gateway::{
    message::{FdSource, MessageBuf, MessageError, Write},
    registry::{ClientObjects, ObjectRegistry},
};

use std::intrinsics::discriminant_value;

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
            .and_then(|f| f(self, args, state))
    }
}

pub struct DispatchState<'a> {
    pub fds: FdSource<'a>,
    pub send_buf: &'a mut MessageBuf<Write>,
    pub registry: &'a mut ObjectRegistry,
    pub objects: &'a mut ClientObjects,
}
