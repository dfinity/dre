use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use fetching::{curl_fetcher::CurlFetcherConfig, sparse_checkout_fetcher::SparseCheckoutFetcherConfig};
use humantime::parse_duration;
use prometheus_http_query::Client;
use slog::{info, o, warn, Drain, Level, Logger};
use tokio::select;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{actions::ActionExecutor, calculation::calculate_progress, registry_wrappers::sync_wrap};

mod actions;
mod calculation;
mod fetching;
mod registry_wrappers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let target_network = ic_management_types::Network::new(args.network.clone(), &args.nns_urls)
        .await
        .expect("Failed to create network");
    let logger = make_logger(args.log_level.clone().into());
    let prometheus_endpoint = target_network.get_prometheus_endpoint();

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

    let executor = match args.private_key_pem {
        Some(path) => ActionExecutor::new(args.neuron_id, path, target_network.clone(), false, Some(&logger)).await?,
        None => ActionExecutor::test(target_network.clone(), Some(&logger)).await?,
    };

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

        info!(logger, "Syncing registry for network '{}'", target_network);
        let maybe_registry_state = select! {
            res = sync_wrap(logger.clone(), args.targets_dir.clone(), target_network.clone()) => res,
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

        // Get blessed replica versions for later
        let blessed_versions = match registry_state.get_blessed_replica_versions().await {
            Ok(versions) => versions,
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
            info!(logger, "Rollout completed");
            token.cancel();
            break;
        }
        info!(logger, "Calculated actions: {:#?}", actions);
        match executor.execute(&actions, &blessed_versions) {
            Ok(()) => info!(logger, "Actions taken successfully"),
            Err(e) => warn!(logger, "{:?}", e),
        };
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

    // Target network. Can be one of: "mainnet", "staging", or an arbitrary "<testnet>" name
    #[clap(long, env = "NETWORK", default_value = "mainnet")]
    network: String,

    // NNS_URLs for the target network, comma separated.
    // The argument is mandatory for testnets, and is optional for mainnet and staging
    #[clap(long, env = "NNS_URLS", aliases = &["registry-url", "nns-url"], value_delimiter = ',')]
    pub nns_urls: Vec<Url>,

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

    #[clap(
        long = "private-key-pem",
        help = r#"
Path to private key pem file that will be used to submit proposals.
If not specified will run in dry-run mode.
        "#
    )]
    private_key_pem: Option<String>,

    #[clap(
        long = "neuron-id",
        help = r#"
Neuron id that corresponds to the key that is in private key pem.
By default is 0.
    "#,
        default_value = "0"
    )]
    neuron_id: u64,

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
