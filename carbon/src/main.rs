#![feature(core_intrinsics)]

mod gateway;
mod protocol;

fn main() {
    env_logger::init();

    log::info!("Starting carbon...");

    let mut gateway = gateway::Gateway::new();
    gateway.run();
}
