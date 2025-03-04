use std::{fmt::Display, path::PathBuf, str::FromStr, sync::Arc};

use axum_otel_metrics::HttpMetricsLayerBuilder;
use clap::Parser;
use metrics::Metrics;
use server::Server;
use slog::{info, o, Drain, Logger};
use storage::{file::FileStorage, in_memory::InMemoryStorage, Storage};
use supervisor::TargetSupervisor;
use tokio_util::sync::CancellationToken;

mod metrics;
mod server;
mod storage;
mod supervisor;

fn main() {
    let logger = make_logger();
    let args = CliArgs::parse();
    info!(logger, "Starting discovery with parameters: {:?}", args);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let token = CancellationToken::new();

    let storage = get_storage_impl(args.mode, logger.clone());
    let storage_sync_handle = storage.sync(runtime.handle().clone(), token.clone());

    let metrics_layer = HttpMetricsLayerBuilder::new().build();
    let metrics = Metrics::new();

    let target_supervisor = TargetSupervisor::new(logger.clone(), token.clone(), metrics.clone(), storage.clone(), runtime.handle().clone());
    runtime.block_on(target_supervisor.start_cached_targets());

    let server = Server::new(logger.clone(), token.clone());
    let server_handle = runtime.spawn(server.run(metrics_layer, target_supervisor.clone()));

    let _ = runtime.block_on(tokio::signal::ctrl_c());
    info!(logger, "Received shutdown in main thread");

    token.cancel();

    // Join all the jobs that watch the targets
    runtime.block_on(target_supervisor.stop_cached_targets());

    // Join the sync thread
    let _ = runtime.block_on(storage_sync_handle);

    // Join the server thread
    let _ = runtime.block_on(server_handle);
}

fn make_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).chan_size(8192).build();
    Logger::root(drain.fuse(), o!())
}

#[derive(Clone, Debug)]
enum StorageMode {
    /// In memory structure for storing the targets
    /// good for testing.
    InMemory,
    /// Periodically write cache to disk which saves
    /// targets across restarts.
    File { path: PathBuf },
}

impl From<&str> for StorageMode {
    fn from(value: &str) -> Self {
        let trimmed = value.trim();
        match trimmed.is_empty() {
            true => StorageMode::InMemory,
            false => StorageMode::File {
                path: PathBuf::from_str(value).unwrap(),
            },
        }
    }
}

fn get_storage_impl(storage_mode: StorageMode, logger: Logger) -> Arc<dyn Storage> {
    match storage_mode {
        StorageMode::InMemory => Arc::new(InMemoryStorage::new()),
        StorageMode::File { path } => Arc::new(FileStorage::new(path, logger)),
    }
}

impl Display for StorageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Parser, Debug)]
struct CliArgs {
    /// Storage mode used for general service discovery.
    #[arg(default_value_t = StorageMode::InMemory, long, short)]
    mode: StorageMode,
}
