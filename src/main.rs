mod gateway;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    log::info!("Starting carbon...");

    let gateway = gateway::Gateway::new();
}
