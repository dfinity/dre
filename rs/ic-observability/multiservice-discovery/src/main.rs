use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use std::vec;

use axum_otel_metrics::HttpMetricsLayerBuilder;
use clap::Parser;
use humantime::parse_duration;
use slog::{info, o, Drain, Logger};
use tokio::runtime::Runtime;
use tokio::sync::oneshot::{self};
use url::Url;

use definition::{Definition, DefinitionsSupervisor, StartMode};
use ic_async_utils::shutdown_signal;

use crate::definition::{RunningDefinition, TargetFilterSpec, TestDefinition};
use crate::metrics::{MSDMetrics, RunningDefinitionsMetrics};
use crate::server_handlers::export_prometheus_config_handler::serialize_definitions_to_prometheus_config;
use crate::server_handlers::Server;

mod definition;
mod metrics;
mod server_handlers;

fn main() {
    let rt = Runtime::new().unwrap();
    let log = make_logger();
    let shutdown_signal = shutdown_signal(log.clone());
    let cli_args = CliArgs::parse();

    fn get_mainnet_definition(cli_args: &CliArgs, log: Logger) -> Definition {
        Definition::new(
            vec![cli_args.nns_url.clone()],
            cli_args.targets_dir.clone(),
            "mercury".to_string(),
            log.clone(),
            None,
            cli_args.poll_interval,
            cli_args.registry_query_timeout,
        )
    }

    if cli_args.render_prom_targets_to_stdout {
        async fn sync(cli_args: &CliArgs, log: &Logger, shutdown_signal: impl futures_util::Future<Output = ()>) -> Option<RunningDefinition> {
            let def = get_mainnet_definition(cli_args, log.clone());
            let test_def = TestDefinition::new(def, RunningDefinitionsMetrics::new());
            let sync_fut = test_def.sync_and_stop(cli_args.skip_update_local_registry);
            tokio::select! {
                _ = sync_fut => {
                    info!(log, "Synchronization done");
                    Some(test_def.running_def)
                },
                _ = shutdown_signal => {
                    test_def.running_def.end().await;
                    None
                }
            }
        }
        if let Some(running_def) = rt.block_on(sync(&cli_args, &log, shutdown_signal)) {
            let mut definitions_ref: BTreeMap<String, RunningDefinition> = BTreeMap::new();
            definitions_ref.insert(running_def.name().clone(), running_def);
            let (_, text) = serialize_definitions_to_prometheus_config(definitions_ref, TargetFilterSpec::empty());
            print!("{}", text);
        }
    } else {
        let supervisor = DefinitionsSupervisor::new(
            rt.handle().clone(),
            cli_args.start_without_mainnet,
            cli_args.networks_state_file.clone(),
            make_logger(),
        );
        let (server_stop, server_stop_receiver) = oneshot::channel();

        // Initialize the metrics layer because in the build method the `global::provider`
        // is set. We can use global::meter only after that call.
        let metrics_layer = HttpMetricsLayerBuilder::new().build();
        let metrics = MSDMetrics::new();

        rt.block_on(supervisor.load_or_create_defs(metrics.running_definition_metrics.clone()))
            .unwrap();

        // First check if we should start the mainnet definition so we can
        // serve it right after the server starts.
        if !cli_args.start_without_mainnet {
            rt.block_on(async {
                let _ = supervisor
                    .start(
                        vec![get_mainnet_definition(&cli_args, log.clone())],
                        StartMode::AddToDefinitions,
                        metrics.running_definition_metrics.clone(),
                    )
                    .await;
            });
        }

        //Configure server
        let server_handle = rt.spawn(
            Server::new(
                log.clone(),
                supervisor.clone(),
                cli_args.poll_interval,
                cli_args.registry_query_timeout,
                cli_args.targets_dir.clone(),
                metrics,
            )
            .run(server_stop_receiver, metrics_layer),
        );

        // Wait for shutdown signal.
        rt.block_on(shutdown_signal);

        // Signal server to stop.  Stop happens in parallel with supervisor stop.
        server_stop.send(()).unwrap();

        //Stop all definitions.  End happens in parallel with server stop.
        rt.block_on(supervisor.end());

        // Wait for server to stop.  Should have stopped by now.
        rt.block_on(server_handle).unwrap();
    }
}

fn make_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).chan_size(8192).build();
    Logger::root(drain.fuse(), o!())
}

#[derive(Parser, Debug)]
#[clap(about, version)]
pub struct CliArgs {
    #[clap(
        long = "targets-dir",
        help = r#"
A writeable directory where the registries of the targeted Internet Computer
instances are stored.

Mainnet (mercury) directory will be created and initialized if no --start-without-mainnet
is provided.

"#
    )]
    targets_dir: PathBuf,

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
    long = "query-request-timeout",
    default_value = "5s",
    value_parser = parse_duration,
    help = r#"
The HTTP-request timeout used when quering for registry updates.

"#
    )]
    registry_query_timeout: Duration,

    #[clap(
        long = "nns-url",
        default_value = "https://ic0.app",
        help = r#"
NNS-url to use for syncing the registry version.
"#
    )]
    nns_url: Url,

    #[clap(
        long = "start-without-mainnet",
        default_value = "false",
        action,
        help = r#"
Start the discovery without the IC Mainnet target.
"#
    )]
    start_without_mainnet: bool,

    #[clap(
        long = "render-prom-targets-to-stdout",
        default_value = "false",
        action,
        help = r#"
Do not run the server; instead, sync and (after syncing) output
the Prometheus targets of mainnet as a JSON structure on stdout.
"#
    )]
    render_prom_targets_to_stdout: bool,

    #[clap(
        long = "skip-update-local-registry",
        default_value = "false",
        action,
        help = r#"
Used for testing: Whether to skip the update of the local mainnet registry.
"#
    )]
    skip_update_local_registry: bool,

    #[clap(
        long = "networks-state-file",
        default_value = None,
        action,
        help = r#"
Preload networks definitions from file path. In case the file does not
exist, it will be created.
"#
    )]
    networks_state_file: Option<PathBuf>,
}
