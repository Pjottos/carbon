use super::message::MessageError;

use std::{collections::VecDeque, os::unix::io::RawFd};

pub trait Interface {
    fn name(&self) -> &'static str;
    fn version(&self) -> u32;
    fn dispatch(
        &mut self,
        opcode: u16,
        args: &[u32],
        fds: &mut VecDeque<RawFd>,
    ) -> Result<(), MessageError>;
}
