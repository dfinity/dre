use std::{path::PathBuf, time::Duration};

use clap::{ArgAction, Parser};
use downloader_loop::run_downloader_loop;
use humantime::parse_duration;
use slog::{info, o, Drain, Logger};
use tokio::{
    runtime::Runtime,
    signal::unix::{signal, SignalKind},
};
use url::Url;

mod downloader_loop;

fn main() {
    let logger = make_logger();
    let rt = Runtime::new().unwrap();
    let shutdown_signal = async {
        let log = logger.clone();
        let mut sig_int = signal(SignalKind::interrupt()).expect("failed to install SIGINT signal handler");
        let mut sig_term = signal(SignalKind::terminate()).expect("failed to install SIGTERM signal handler");

        tokio::select! {
            _ = sig_int.recv() => {
                info!(log, "Caught SIGINT");
            }
            _ = sig_term.recv() => {
                info!(log, "Caught SIGTERM");
            }
        }
    };
    let cli_args = CliArgs::parse();
    let (stop_signal_sender, stop_signal_rcv) = crossbeam::channel::bounded::<()>(0);

    info!(logger, "Starting downloader loop"; "cli_args" => ?cli_args);

    let downloader_handle = rt.spawn(run_downloader_loop(logger.clone(), cli_args, stop_signal_rcv));

    rt.block_on(shutdown_signal);
    info!(logger, "Received shutdown signal, shutting down ...");

    stop_signal_sender.send(()).unwrap();

    let _ = rt.block_on(downloader_handle);
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
        long = "output-dir",
        help = r#"
A writeable directory where the outptu of the targeted Internet Computer
instances are stored.
"#
    )]
    pub output_dir: PathBuf,

    #[clap(
    long = "poll-interval",
    default_value = "30s",
    value_parser = parse_duration,
    help = r#"
The interval at which targets are polled for updates.

"#
    )]
    pub poll_interval: Duration,

    #[clap(
    long = "query-request-timeout",
    default_value = "15s",
    value_parser = parse_duration,
    help = r#"
The HTTP-request timeout used when quering for registry updates.

"#
    )]
    pub registry_query_timeout: Duration,

    #[clap(
        long = "nns-urls",
        default_value = "https://ic0.app",
        help = r#"
NNS url to use for syncing the targets.
"#
    )]
    pub nns_urls: Vec<Url>,

    #[clap(
        long = "sd-url",
        default_value = "https://sns-api.internetcomputer.org/api/v1/snses",
        help = r#"
Service Discovery url to use for syncing the targets.
"#
    )]
    pub sd_url: Url,

    #[clap(long = "script-path", help = "Path for the script file")]
    script_path: String,

    #[clap(long = "cursors-folder", help = "Path for cursors")]
    cursors_folder: String,

    #[clap(
                long = "restart-on-exit",
                help = "Restart on respawn",
                action = ArgAction::SetTrue,
                default_value = "false"
            )]
    restart_on_exit: bool,

    #[clap(long = "include-stderr", help = "Include stderr", action = ArgAction::SetTrue, default_value = "false")]
    include_stderr: bool,
}
