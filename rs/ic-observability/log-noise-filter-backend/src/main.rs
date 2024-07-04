use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use clap::Parser;
use handlers::WholeState;
use slog::{info, o, warn, Drain, Level, Logger};

use crate::handlers::Server;

mod handlers;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let logger = make_logger(from_str_to_log(&cli.log_level));
    info!(logger, "Running with following args: {:?}", cli);

    let socket = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), cli.port);
    info!(logger, "Running noise filter manager {}", socket);

    let (global_rate, criteria) = load_state(&cli.state_file, &logger).await;

    let server = Server::new(logger.clone(), global_rate, criteria.into_values().collect(), Some(cli.state_file));
    server.run(socket).await;

    info!(logger, "Noise filter manager stopped");
}

async fn load_state(path: &PathBuf, logger: &Logger) -> (u64, BTreeMap<u32, String>) {
    if path.exists() {
        let content = tokio::fs::read_to_string(path).await.unwrap();
        let maybe_state = serde_json::from_str::<WholeState>(&content);
        match maybe_state {
            Ok(state) => return (state.rate, state.criteria),
            Err(e) => warn!(
                logger,
                "Failed to deserialize state file {}, will remove it and recreate. The error was: {:?}",
                path.display(),
                e
            ),
        }
    }

    let default = WholeState::default();
    tokio::fs::write(path, serde_json::to_string_pretty(&default).unwrap()).await.unwrap();
    (default.rate, default.criteria)
}

fn make_logger(level: Level) -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let full_format = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog::Filter::new(full_format, move |record: &slog::Record| record.level().is_at_least(level)).fuse();
    let drain = slog_async::Async::new(drain).chan_size(8192).build();
    Logger::root(drain.fuse(), o!())
}

#[derive(Parser, Debug)]
struct Cli {
    #[clap(
        long,
        default_value = "info",
        help = r#"
Log level to use for running. You can use standard log levels 'info',
'critical', 'error', 'warning', 'trace', 'debug'

"#
    )]
    log_level: String,

    #[clap(long, default_value = "8080", help = "Port to use for running the api")]
    port: u16,

    #[clap(long, help = "State file used to sync across restarts")]
    state_file: PathBuf,
}

fn from_str_to_log(value: &str) -> Level {
    match value {
        "info" => Level::Info,
        "critical" => Level::Critical,
        "error" => Level::Error,
        "warning" => Level::Warning,
        "trace" => Level::Trace,
        "debug" => Level::Debug,
        _ => panic!("Unsupported level: {}", value),
    }
}
