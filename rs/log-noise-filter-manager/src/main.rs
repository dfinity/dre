use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use clap::Parser;
use slog::{info, o, Drain, Level, Logger};

use crate::handlers::Server;

mod handlers;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let logger = make_logger(from_str_to_log(&cli.log_level));
    info!(logger, "Running with following args: {:?}", cli);

    let socket = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), cli.port);
    info!(logger, "Running noise filter manager {}", socket);

    let server = Server::new(logger.clone(), cli.file_path);
    server.run(socket, cli.reroute_unmached, cli.inputs).await;

    info!(logger, "Noise filter manager stopped");
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

    #[clap(
        long,
        help = "File path to the vector config in toml used for the routing configuration"
    )]
    file_path: PathBuf,

    #[clap(
        long,
        help = "Explained: https://vector.dev/docs/reference/configuration/transforms/route/#reroute_unmatched"
    )]
    reroute_unmached: String,

    #[clap(long, help = "All inputs that should be linked to this transform")]
    inputs: Vec<String>,
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
