use std::{path::PathBuf, time::Duration};

use clap::Parser;
use humantime::parse_duration;
use ic_management_types::Network;
use registry_wrappers::inital_sync_wrap;
use slog::{info, o, Drain, Level, Logger};
use tokio::sync::broadcast;

mod registry_wrappers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let logger = make_logger(args.log_level.clone().into());

    let shutdown = tokio::signal::ctrl_c();
    info!(logger, "Running release controller with arguments: {:#?}", args);
    info!(logger, "Syncing registry for network '{:?}'", args.network);

    match inital_sync_wrap(logger.clone(), args.targets_dir, args.network).await {
        Ok(registry_wrappers::InitSyncCompletionStatus::Completed) => {
            info!(logger, "Initial sync completed. Proceeding...")
        }
        Ok(registry_wrappers::InitSyncCompletionStatus::ShutdownRequested) => {
            info!(logger, "Received shutdown signal");
            return Ok(());
        }
        Err(e) => return Err(anyhow::anyhow!("Error during inital sync: {:?}", e)),
    }

    let futures = vec![];
    let (sender, mut receiver_sync) = broadcast::channel(1);

    futures.push(tokio::spawn(registry_wrappers::poll(
        logger.clone(),
        args.targets_dir,
        args.poll_interval,
        receiver_sync,
    )));

    shutdown
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't receive shutdown: {:?}", e))?;
    info!(logger, "Received shutdown signal");

    Ok(())
}

fn make_logger(level: Level) -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let full_format = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog::Filter::new(full_format, move |record: &slog::Record| {
        record.level().is_at_least(level)
    })
    .fuse();
    let drain = slog_async::Async::new(drain).chan_size(8192).build();
    Logger::root(drain.fuse(), o!())
}

#[derive(Parser, Debug)]
#[clap(about, version)]
struct Cli {
    #[clap(
        long = "targets-dir",
        help = r#"
A writeable directory where the registries of the targeted Internet Computer
instances are stored.
"#
    )]
    targets_dir: PathBuf,

    #[clap(
        long,
        default_value = "mainnet",
        help = r#"
Target network to observe and update with the controller. 
Can be one of:
    1. mainnet,
    2. staging,
    3. arbitrary nns url

"#
    )]
    network: Network,

    #[clap(
        long,
        default_value = "info",
        help = r#"
Log level to use for running. You can use standard log levels 'info',
'critical', 'error', 'warning', 'trace', 'debug'

"#
    )]
    log_level: LogLevel,

    #[clap(
        long = "poll-interval",
        default_value = "30s",
        value_parser = parse_duration,
        help = r#"
The interval at which ICs are polled for updates.
    
"#
        )]
    poll_interval: Duration,
}

#[derive(Debug, Clone)]
enum LogLevel {
    Info,
    Critical,
    Error,
    Warning,
    Trace,
    Debug,
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value {
            "info" => Self::Info,
            "critical" => Self::Critical,
            "error" => Self::Error,
            "warning" => Self::Warning,
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            _ => panic!("Unknown log level"),
        }
    }
}

impl From<LogLevel> for Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Info => Self::Info,
            LogLevel::Critical => Self::Critical,
            LogLevel::Error => Self::Error,
            LogLevel::Warning => Self::Warning,
            LogLevel::Trace => Self::Trace,
            LogLevel::Debug => Self::Debug,
        }
    }
}
