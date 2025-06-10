use clap::CommandFactory;
use clap::Parser;
use dotenv::dotenv;
use dre::commands::{
    main_command::{MainCommand, Subcommands},
    upgrade::{UpdateStatus, Upgrade},
};
use dre::ctx::DreContext;
use dre::exe::ExecutableCommand;
use log::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    let curr_version = env!("CARGO_PKG_VERSION");
    if curr_version != "0.0.0" {
        info!("Running version {}", curr_version);
    }

    dotenv().ok();

    let args = MainCommand::parse();

    let mut cmd = MainCommand::command();
    args.validate(&args.global_args, &mut cmd);

    if let Subcommands::Upgrade(upgrade) = args.subcommands {
        let response = upgrade.run(curr_version).await?;
        match response {
            UpdateStatus::NoUpdate => info!("Already running the latest version"),
            UpdateStatus::NewVersion(_) => unreachable!("Shouldn't happen"),
            UpdateStatus::Updated(v) => info!("Upgraded: {} -> {}", curr_version, v),
        }
        return Ok(());
    }

    let ctx = DreContext::from_args(&args.global_args, args.subcommands.require_auth(), args.neuron_override()).await?;

    let r = args.execute(ctx).await;

    let handle = Upgrade::default().check(curr_version);
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
    pretty_env_logger::try_init_timed_custom_env("LOG_LEVEL").expect("Failed to initialize logger");
}
