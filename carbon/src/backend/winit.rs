use crate::backend::Backend;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::{run_return::EventLoopExtRunReturn, unix::EventLoopExtUnix},
    window::{Window, WindowBuilder},
};

use std::thread;

pub struct Winit {
    window_thread: thread::JoinHandle<()>,
}

impl Winit {
    pub fn new() -> Self {
        let window_thread = thread::spawn(|| {
            let mut event_loop = EventLoop::new_any_thread();
            let window = WindowBuilder::new()
                .with_title("carbon")
                .build(&event_loop)
                .expect("failed to build window");
            run_event_loop(&mut event_loop, window);
        });

        Self { window_thread }
    }
}

impl Backend for Winit {}

fn run_event_loop(event_loop: &mut EventLoop<()>, window: Window) {
    // Window needs to be dropped inside the event loop, otherwise it will stay open
    // See: https://github.com/rust-windowing/winit/issues/2345
    let mut window = Some(window);
    event_loop.run_return(move |event, _window_target, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
                window.take();
            }
            _ => (),
        },
        Event::RedrawRequested(_) => (),
        Event::RedrawEventsCleared => {
            *control_flow = ControlFlow::Wait;
        }
        _ => (),
    });
}
