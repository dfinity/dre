use std::path::PathBuf;

use clap::Parser;
use download_loop::download_loop;
use slog::{info, o, Drain, Level, Logger};
use url::Url;

mod download_loop;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let logger = make_logger(from_str_to_log(&cli.log_level));
    info!(logger, "Running with following args: {:?}", cli);

    download_loop(cli.url, logger, cli.path, cli.inputs, cli.rate, cli.transform_id).await
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

    #[clap(long, help = "Url for the backend")]
    url: Url,

    #[clap(long, help = "Path to where the output should be generated")]
    path: PathBuf,

    #[clap(
        long,
        help = "Rate of the matched messages that should be let through. It will be 1/rate",
        default_value = "100"
    )]
    rate: u64,

    #[clap(long, help = "Inputs that will be linked to this transform")]
    inputs: Vec<String>,

    #[clap(long, help = "Transform id", default_value = "sample-ic-logs-transform")]
    transform_id: String,
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
