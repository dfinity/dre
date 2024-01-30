use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use futures_util::FutureExt;
use humantime::parse_duration;
use slog::{o, Drain, Logger};
use tokio::runtime::Runtime;
use tokio::sync::oneshot::{self};
use url::Url;

use definition::{Definition, DefinitionsSupervisor, StartMode};
use ic_async_utils::shutdown_signal;
use ic_management_types::Network;

use crate::server_handlers::Server;

mod definition;
mod server_handlers;

fn main() {
    let rt = Runtime::new().unwrap();
    let log = make_logger();
    let shutdown_signal = shutdown_signal(log.clone()).shared();
    let cli_args = CliArgs::parse();

    let supervisor = DefinitionsSupervisor::new(rt.handle().clone(), cli_args.start_without_mainnet);
    let (server_stop, server_stop_receiver) = oneshot::channel();

    //Configure server
    let server_handle = rt.spawn(
        Server::new(
            log.clone(),
            supervisor.clone(),
            cli_args.poll_interval,
            cli_args.registry_query_timeout,
            cli_args.targets_dir.clone(),
        )
        .run(server_stop_receiver),
    );
    if !cli_args.start_without_mainnet {
        rt.block_on(async {
            let _ = supervisor
                .start(
                    vec![get_mainnet_definition(&cli_args, log.clone())],
                    StartMode::AddToDefinitions,
                )
                .await;
        });
    }

    // Wait for shutdown signal.
    rt.block_on(shutdown_signal);

    // Signal server to stop.  Stop happens in parallel with supervisor stop.
    server_stop.send(()).unwrap();

    //Stop all definitions.  End happens in parallel with server stop.
    rt.block_on(supervisor.end());

    // Wait for server to stop.  Should have stopped by now.
    rt.block_on(server_handle).unwrap();
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
}

fn get_mainnet_definition(cli_args: &CliArgs, log: Logger) -> Definition {
    Definition::new(
        vec![cli_args.nns_url.clone()],
        cli_args.targets_dir.clone(),
        Network::Mainnet.legacy_name(),
        log.clone(),
        None,
        cli_args.poll_interval,
        cli_args.registry_query_timeout,
    )
}
