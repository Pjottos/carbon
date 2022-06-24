use crate::input::InputSink;

use std::{io, os::unix::io::RawFd};

mod winit;
pub use self::winit::Winit;

pub trait Backend {
    fn input_fd(&self) -> RawFd;
    fn drain_input(&mut self, sink: &mut InputSink) -> io::Result<()>;
}
