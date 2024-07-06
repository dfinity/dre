use clap::Parser;
use commands::{
    upgrade::{UpdateStatus, Upgrade},
    Args, ExecutableCommand,
};
use ctx::DreContext;
use dotenv::dotenv;
use log::{info, warn};

mod auth;
mod commands;
mod ctx;
mod ic_admin;
mod operations;
mod runner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    init_logger();
    let version = env!("CARGO_PKG_VERSION");
    info!("Running version {}", version);

    dotenv().ok();

    let args = Args::parse();

    if let commands::Subcommands::Upgrade(upgrade) = args.subcommands {
        let response = upgrade.run().await?;
        match response {
            UpdateStatus::NoUpdate => info!("Running the latest version"),
            UpdateStatus::NewVersion(_) => unreachable!("Shouldn't happen"),
            UpdateStatus::Updated(v) => info!("Upgraded: {} -> {}", version, v),
        }
        return Ok(());
    }

    let ctx = DreContext::from_args(&args).await?;

    let r = args.execute(ctx).await;

    let handle = Upgrade {}.check();
    let maybe_update_status = handle.await?;
    match maybe_update_status {
        Ok(s) => match s {
            UpdateStatus::NoUpdate => {}
            UpdateStatus::NewVersion(v) => info!("There is a new version '{}' available. Run 'dre upgrade' to upgrade", v),
            UpdateStatus::Updated(_) => unreachable!("Shouldn't happen"),
        },
        Err(e) => warn!("There was an error while checking for new updates: {:?}", e),
    }

    r
}

fn init_logger() {
    match std::env::var("RUST_LOG") {
        Ok(val) => std::env::set_var("LOG_LEVEL", val),
        Err(_) => {
            if std::env::var("LOG_LEVEL").is_err() {
                // Default logging level is: info generally, warn for mio and actix_server
                // You can override defaults by setting environment variables
                // RUST_LOG or LOG_LEVEL
                std::env::set_var("LOG_LEVEL", "info,mio::=warn,actix_server::=warn")
            }
        }
    }
    pretty_env_logger::init_custom_env("LOG_LEVEL");
}
