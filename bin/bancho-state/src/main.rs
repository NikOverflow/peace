#[allow(unused_imports)]
#[macro_use]
extern crate peace_logs;

#[macro_use]
extern crate peace_rpc;

pub mod app;
pub mod rpc;

pub use app::*;
pub use rpc::*;

use peace_snapshot::SnapshopConfig;

pub async fn run(
    cfg: std::sync::Arc<BanchoStateConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a new instance of the `App.
    let app = App::initialize(cfg.clone()).await;

    // Start serving the RPC server with the `App` instance.
    peace_rpc::server::serve(app.clone()).await;

    if cfg.bancho_state_snapshot.should_save_snapshot() {
        let _ = app
            .bancho_state_service
            .save_service_snapshot(
                cfg.bancho_state_snapshot.snapshot_type(),
                cfg.bancho_state_snapshot.snapshot_path(),
            )
            .await;
    }

    Ok(())
}

/// The main entry point of the application.
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    tools::main_startup_info!();

    let cfg = BanchoStateConfig::get();
    // Initialize the logger.
    peace_logs::init(&cfg.frame_cfg);

    // Initialize runtime and run app.
    peace_runtime::runtime(&cfg.runtime_cfg).unwrap().block_on(run(cfg))
}
