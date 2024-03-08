use std::{path::PathBuf, str::FromStr, time::Duration};

use clap::{Parser, Subcommand};
use fetching::{curl_fetcher::CurlFetcherConfig, sparse_checkout_fetcher::SparseCheckoutFetcherConfig};
use humantime::parse_duration;
use ic_management_types::Network;
use prometheus_http_query::Client;
use slog::{info, o, warn, Drain, Level, Logger};
use tokio::select;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{registry_wrappers::sync_wrap, rollout_schedule::calculate_progress};

mod fetching;
mod registry_wrappers;
mod rollout_schedule;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let logger = make_logger(args.log_level.clone().into());
    let prometheus_endpoint = match &args.network {
        Network::Mainnet => Url::from_str("https://victoria.ch1-obs1.dfinity.network")
            .map_err(|e| anyhow::anyhow!("Couldn't parse url: {:?}", e))?,
        Network::Staging => Url::from_str("https://victoria.ch1-obsstage1.dfinity.network")
            .map_err(|e| anyhow::anyhow!("Couldn't parse url: {:?}", e))?,
        Network::Url(url) => url.clone(),
    };
    let prometheus_endpoint = prometheus_endpoint
        .join("select/0/prometheus")
        .map_err(|e| anyhow::anyhow!("Couldn't append victoria prometheus endpoint: {:?}", e))?;

    let client = Client::try_from(prometheus_endpoint.to_string())
        .map_err(|e| anyhow::anyhow!("Couldn't create prometheus client: {:?}", e))?;

    let shutdown = tokio::signal::ctrl_c();
    let token = CancellationToken::new();
    info!(logger, "Running release controller with arguments: {:#?}", args);

    let shutdown_logger = logger.clone();
    let shutdown_token = token.clone();
    let shutdown_handle = tokio::spawn(async move {
        select! {
            _ = shutdown => shutdown_token.cancel(),
            _ = shutdown_token.cancelled() => {}
        }
        info!(shutdown_logger, "Received shutdown");
    });

    let fetcher = fetching::resolve(args.subcommand, logger.clone()).await?;

    let mut interval = tokio::time::interval(args.poll_interval);
    let mut should_sleep = false;
    loop {
        if should_sleep {
            select! {
                _ = token.cancelled() => break,
                tick = interval.tick() => info!(logger, "Running loop @ {:?}", tick),
            }
        } else if token.is_cancelled() {
            break;
        }
        should_sleep = true;

        info!(logger, "Syncing registry for network '{:?}'", args.network);
        let maybe_registry_state = select! {
            res = sync_wrap(logger.clone(), args.targets_dir.clone(), args.network.clone()) => res,
            _ = token.cancelled() => break,
        };
        let registry_state = match maybe_registry_state {
            Ok(state) => {
                info!(logger, "Syncing registry completed");
                state
            }
            Err(e) => {
                warn!(logger, "{:?}", e);
                should_sleep = false;
                continue;
            }
        };

        info!(logger, "Fetching rollout index");
        let index = match fetcher.fetch().await {
            Ok(index) => {
                info!(logger, "Fetching of new index complete");
                index
            }
            Err(e) => {
                warn!(logger, "{:?}", e);
                should_sleep = false;
                continue;
            }
        };

        // Calculate what should be done
        info!(logger, "Calculating the progress of the current release");
        let actions = match calculate_progress(&logger, index, &client, registry_state).await {
            Ok(actions) => actions,
            Err(e) => {
                warn!(logger, "{:?}", e);
                continue;
            }
        };
        info!(logger, "Calculating completed");

        if actions.is_empty() {
            info!(logger, "No actions needed, sleeping");
            continue;
        }
        info!(logger, "Calculated actions: {:#?}", actions);
        // Apply changes
        token.cancel();
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
        long = "prometheus-endpoint",
        help = r#"
Optional url of prometheus endpoint to use for querying bake time.
If not specified it will take following based on 'Network' values:
        1. Mainnet => https://victoria.ch1-obs1.dfinity.network
        2. Staging => https://victoria.ch1-obsstage1.dfinity.network
        3. arbitrary nns url => must be specified or will error

"#
    )]
    victoria_url: Option<String>,

    #[clap(subcommand)]
    pub(crate) subcommand: Commands,
}

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    Git(SparseCheckoutFetcherConfig),
    Curl(CurlFetcherConfig),
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
