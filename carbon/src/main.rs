#![feature(core_intrinsics)]

mod backend;
mod gateway;
mod protocol;

fn main() {
    env_logger::init();

    log::info!("Starting carbon...");

    let _backend = backend::Winit::new();

    let mut gateway = gateway::Gateway::new();
    gateway.run();
}
