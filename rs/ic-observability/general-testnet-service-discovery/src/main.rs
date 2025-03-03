use std::{fmt::Display, path::PathBuf, str::FromStr};

use clap::Parser;
use slog::{info, o, Drain, Logger};
use storage::{file::FileStorage, in_memory::InMemoryStorage, Storage};
use tokio_util::sync::CancellationToken;

mod storage;

fn main() {
    let logger = make_logger();
    let args = CliArgs::parse();
    info!(logger, "Starting discovery with parameters: {:?}", args);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let token = CancellationToken::new();

    let storage = get_storage_impl(args.mode, logger.clone());
    let storage_sync_handle = storage.sync(runtime.handle().clone(), token.clone());

    let _ = runtime.block_on(tokio::signal::ctrl_c());
    info!(logger, "Received shutdown in main thread");

    token.cancel();

    // Join the sync thread
    let _ = runtime.block_on(storage_sync_handle);
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

fn get_storage_impl(storage_mode: StorageMode, logger: Logger) -> Box<dyn Storage> {
    match storage_mode {
        StorageMode::InMemory => Box::new(InMemoryStorage::new()),
        StorageMode::File { path } => Box::new(FileStorage::new(path, logger)),
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
