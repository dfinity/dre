use std::{path::PathBuf, time::Duration};

use clap::Parser;
use humantime::parse_duration;
use ic_management_types::Network;
use registry_wrappers::inital_sync_wrap;
use slog::{info, o, Drain, Level, Logger};
use tokio::{
    runtime::{Handle, Runtime},
    sync::broadcast,
};

use crate::{git_sync::watch_git, release_file_watcher::handle_file_update};

mod git_sync;
mod registry_wrappers;
mod release_file_watcher;

fn main() -> anyhow::Result<()> {
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle().clone();

    runtime.block_on(async_main(handle))
}

async fn async_main(rt: Handle) -> anyhow::Result<()> {
    let args = Cli::parse();
    let logger = make_logger(args.log_level.clone().into());

    let shutdown = tokio::signal::ctrl_c();
    info!(logger, "Running release controller with arguments: {:#?}", args);
    info!(logger, "Syncing registry for network '{:?}'", args.network);

    match inital_sync_wrap(logger.clone(), args.targets_dir.clone(), args.network).await {
        Ok(registry_wrappers::InitSyncCompletionStatus::Completed) => {
            info!(logger, "Initial sync completed. Proceeding...")
        }
        Ok(registry_wrappers::InitSyncCompletionStatus::ShutdownRequested) => {
            info!(logger, "Received shutdown signal");
            return Ok(());
        }
        Err(e) => return Err(anyhow::anyhow!("Error during inital sync: {:?}", e)),
    }

    // Special case for this thread because it was developed with mostly sync specific code
    let mut futures_crossbeam = vec![];
    let (sender_poll, receiver_poll) = crossbeam_channel::bounded(1);
    let logger_cloned = logger.clone();
    let rt_cloned = rt.clone();
    futures_crossbeam.push(std::thread::spawn(move || {
        registry_wrappers::poll(
            logger_cloned,
            args.targets_dir,
            args.poll_interval,
            receiver_poll,
            rt_cloned,
        )
    }));

    // Tokio threads
    let (sender, receiver_sync) = broadcast::channel(1);

    // Channel for sending updates of release file
    let (file_notification_sender, file_notification_receiver) = broadcast::channel(1);

    let mut futures_tokio = vec![];
    futures_tokio.push(rt.spawn(watch_git(
        logger.clone(),
        args.git_repo_path.clone(),
        args.poll_interval,
        receiver_sync,
        args.git_repo_url,
        args.release_index.clone(),
        file_notification_sender,
    )));

    // Configure file watcher
    let shutdown_file_watcher = sender.subscribe();
    futures_tokio.push(rt.spawn(handle_file_update(
        logger.clone(),
        file_notification_receiver,
        shutdown_file_watcher,
    )));

    shutdown
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't receive shutdown: {:?}", e))?;
    info!(logger, "Received shutdown signal");

    sender
        .send(())
        .map_err(|e| anyhow::anyhow!("Couldn't send stop signal: {:?}", e))?;
    sender_poll
        .send(())
        .map_err(|e| anyhow::anyhow!("Couldn't send stop signal to poll loop: {:?}", e))?;

    for future in futures_crossbeam {
        future
            .join()
            .map_err(|e| anyhow::anyhow!("Couldn't join crossbeam thread: {:?}", e))??;
    }

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
