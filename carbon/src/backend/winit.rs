use crate::{
    backend::Backend,
    input::{InputSink, SeatId},
    protocol::wl_seat::Capability,
};

use nix::{
    sys::eventfd::{eventfd, EfdFlags},
    unistd::{close, read, write},
};
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    platform::{run_return::EventLoopExtRunReturn, unix::EventLoopExtUnix},
    window::WindowBuilder,
};

use std::{
    io,
    mem::drop,
    os::unix::io::RawFd,
    sync::mpsc::{self, TryRecvError},
    thread,
};

pub struct Winit {
    input_rx: mpsc::Receiver<DeviceEvent>,
    input_fd: RawFd,
    proxy: EventLoopProxy<BackendDropped>,
    seat_id: Option<SeatId>,
    closed: bool,
}

impl Drop for Winit {
    fn drop(&mut self) {
        let _ = self.proxy.send_event(BackendDropped);
        let _ = close(self.input_fd);
    }
}

impl Winit {
    pub fn new() -> Self {
        let (input_tx, input_rx) = mpsc::channel();
        let (proxy_tx, proxy_rx) = mpsc::sync_channel(1);
        let input_fd = eventfd(0, EfdFlags::EFD_NONBLOCK | EfdFlags::EFD_CLOEXEC)
            .expect("failed to create eventfd");
        thread::spawn(move || run_event_loop(proxy_tx, input_tx, input_fd));
        let proxy = proxy_rx.recv().unwrap();

        Self {
            input_rx,
            input_fd,
            proxy,
            seat_id: None,
            closed: false,
        }
    }
}

impl Backend for Winit {
    fn input_fd(&self) -> RawFd {
        self.input_fd
    }

    fn drain_input(&mut self, sink: &mut InputSink) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }

        let seat_id = *self
            .seat_id
            .get_or_insert_with(|| sink.create_seat(Capability::KEYBOARD | Capability::POINTER));

        let mut counter = 0u64.to_ne_bytes();
        read(self.input_fd, &mut counter)?;

        loop {
            match self.input_rx.try_recv() {
                Ok(event) => match event {
                    DeviceEvent::MouseMotion { delta } => (),
                    DeviceEvent::MouseWheel { delta } => (),
                    DeviceEvent::Motion { axis, value } => (),
                    DeviceEvent::Button { button, state } => match button {
                        1 | 0x110 => log::debug!("left"),
                        2 | 0x112 => log::debug!("middle"),
                        3 | 0x111 => log::debug!("right"),
                        _ => log::warn!("Unknown button event: {:04x} {:?}", button, state),
                    },
                    DeviceEvent::Key(key) => (),
                    _ => (),
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    sink.destroy_seat(seat_id);
                    self.seat_id = None;
                    self.closed = true;
                    break;
                }
            }
        }

        Ok(())
    }
}

struct BackendDropped;

fn run_event_loop(
    proxy_tx: mpsc::SyncSender<EventLoopProxy<BackendDropped>>,
    input_tx: mpsc::Sender<DeviceEvent>,
    input_fd: RawFd,
) {
    let mut event_loop = EventLoop::new_any_thread();
    let proxy = event_loop.create_proxy();
    proxy_tx.send(proxy).unwrap();
    drop(proxy_tx);

    let window = WindowBuilder::new()
        .with_title("carbon")
        .build(&event_loop)
        .expect("failed to build window");
    // Window needs to be dropped inside the event loop, otherwise it will stay open
    // See: https://github.com/rust-windowing/winit/issues/2345
    let mut window = Some(window);

    event_loop.run_return(move |event, _window_target, control_flow| {
        match event {
            Event::NewEvents(_) => *control_flow = ControlFlow::Wait,
            Event::UserEvent(BackendDropped) => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            },
            Event::DeviceEvent { event, .. } => match input_tx.send(event) {
                Ok(_) => {
                    write(input_fd, &1u64.to_ne_bytes()).expect("failed to write input eventfd");
                }
                Err(_) => *control_flow = ControlFlow::Exit,
            },
            Event::RedrawRequested(_) => (),
            _ => (),
        }

        if *control_flow == ControlFlow::Exit {
            window.take();
        }
    });

    // Cause poll for input to destroy the seat
    // Do nothing if the input_fd has been closed
    let _ = write(input_fd, &1u64.to_ne_bytes());
}
