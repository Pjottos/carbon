mod gateway;

fn main() {
    env_logger::init();

    log::info!("Starting carbon...");

    let mut gateway = gateway::Gateway::new();
    gateway.run();
}
