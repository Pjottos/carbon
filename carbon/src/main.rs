use tokio::{runtime, task};

mod gateway;
mod message;

fn main() {
    env_logger::init();

    log::info!("Starting carbon...");

    runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async move {
            let local = task::LocalSet::new();
            local
                .run_until(async move {
                    let gateway = gateway::Gateway::new();
                    gateway.listen().await;
                })
                .await;
        });
}
