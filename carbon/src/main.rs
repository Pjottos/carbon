#![feature(core_intrinsics)]

mod backend;
mod gateway;
mod input;
mod protocol;

fn main() {
    env_logger::init();

    log::info!("Starting carbon...");

    let backend = backend::Winit::new();

    let mut gateway = gateway::Gateway::new(backend);
    gateway.run();
}
