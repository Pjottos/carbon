use crate::gateway::{
    message::{FdSource, MessageBuf, MessageError, Write},
    registry::{ClientObjects, ObjectRegistry},
};

use std::intrinsics::discriminant_value;

mod generated;
pub use generated::Interface;

mod wayland;
pub use wayland::*;
mod xdg_shell;
pub use xdg_shell::*;

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

    #[inline]
    pub fn id(&self) -> u32 {
        discriminant_value(self) as u32
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        generated::INTERFACE_NAMES[discriminant_value(self) as usize]
    }

    #[inline]
    pub fn version(&self) -> u32 {
        generated::INTERFACE_VERSIONS[discriminant_value(self) as usize]
    }
}

pub struct DispatchState<'a> {
    pub fds: FdSource<'a>,
    pub send_buf: &'a mut MessageBuf<Write>,
    pub registry: &'a mut ObjectRegistry,
    pub objects: &'a mut ClientObjects,
}
