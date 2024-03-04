use std::{path::PathBuf, time::Duration};

use clap::Parser;
use humantime::parse_duration;
use ic_management_types::Network;
use slog::{info, o, warn, Drain, Level, Logger};
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{git_sync::sync_git, registry_wrappers::sync_wrap};

mod git_sync;
mod registry_wrappers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let logger = make_logger(args.log_level.clone().into());

    let shutdown = tokio::signal::ctrl_c();
    let token = CancellationToken::new();
    info!(logger, "Running release controller with arguments: {:#?}", args);

    let shutdown_logger = logger.clone();
    let shutdown_token = token.clone();
    let shutdown_handle = tokio::spawn(async move {
        shutdown.await.unwrap();
        info!(shutdown_logger, "Received shutdown");
        shutdown_token.cancel();
    });

    let mut interval = tokio::time::interval(args.poll_interval);
    let mut should_sleep = false;
    loop {
        if should_sleep {
            select! {
                tick = interval.tick() => info!(logger, "Running loop @ {:?}", tick),
                _ = token.cancelled() => break,
            }
        } else if token.is_cancelled() {
            break;
        }
        should_sleep = true;

        // Sync registry
        info!(logger, "Syncing registry for network '{:?}'", args.network);
        match sync_wrap(logger.clone(), args.targets_dir.clone(), args.network.clone()).await {
            Ok(()) => info!(logger, "Syncing registry completed"),
            Err(e) => {
                warn!(logger, "{:?}", e);
                should_sleep = false;
                continue;
            }
        };

        info!(logger, "Syncing git repo");
        match sync_git(&logger, &args.git_repo_path, &args.git_repo_url, &args.release_index).await {
            Ok(()) => info!(logger, "Syncing git repo completed"),
            Err(e) => {
                warn!(logger, "{:?}", e);
                should_sleep = false;
                continue;
            }
        }

        // Read prometheus

        // Read last iteration from disk

        // Calculate what should be done

        // Apply changes

        // Serialize new state to disk
    }
    info!(logger, "Shutdown complete");
    shutdown_handle.await.unwrap();

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

    #[clap(
        long = "git-repo-path",
        help = r#"
The path to the directory that will be used for git sync

"#
    )]
    git_repo_path: PathBuf,

    #[clap(
        long = "git-repo-url",
        default_value = "git@github.com:dfinity/dre.git",
        help = r#"
The url of the repository with which we should sync.

"#
    )]
    git_repo_url: String,

    #[clap(
        long = "release-file-name",
        default_value = "release-index.yaml",
        help = r#"
The fully qualified name of release index file in the git repositry.

"#
    )]
    release_index: String,
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
