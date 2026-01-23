use std::path::PathBuf;
use std::time::Duration;

use axum::Router;
use axum::http::Method;
use axum::routing::{get, post};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use clap::Parser;
use humantime::parse_duration;
use opentelemetry::global;
use prometheus::{Encoder, TextEncoder};
use slog::{Drain, Logger, error, info, o};
use tokio::runtime::Runtime;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::oneshot;
use tower_http::cors::{Any, CorsLayer};
use url::Url;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use cloud_engine_controller_backend::config::AppConfig;
use cloud_engine_controller_backend::handlers::{node_handlers, subnet_handlers, vm_handlers};
use cloud_engine_controller_backend::openapi::ApiDoc;
use cloud_engine_controller_backend::state::AppState;

fn main() {
    let rt = Runtime::new().unwrap();
    let log = make_logger();
    let shutdown_signal = make_shutdown_signal(log.clone());
    let cli_args = CliArgs::parse();
    info!(log, "Starting Cloud Engine Controller Backend with args: {:?}", cli_args);

    // Load application config from file
    let config = match AppConfig::load(&cli_args.config_file) {
        Ok(config) => {
            info!(log, "Loaded config";
                "project_id" => &config.gcp.project_id,
                "zones" => ?config.gcp.zones
            );
            config
        }
        Err(e) => {
            error!(log, "Failed to load config file: {}", e);
            std::process::exit(1);
        }
    };

    let (server_stop_tx, server_stop_rx) = oneshot::channel();

    // Initialize metrics
    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(prometheus::default_registry().clone())
        .build()
        .unwrap();

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder().with_reader(exporter).build();

    global::set_meter_provider(provider.clone());
    let metrics_layer = HttpMetricsLayerBuilder::new().build();

    // Initialize application state
    let state = rt.block_on(async {
        AppState::new(
            log.clone(),
            cli_args.targets_dir.clone(),
            cli_args.nns_url.clone(),
            cli_args.poll_interval,
            cli_args.registry_query_timeout,
            cli_args.gcp_credentials_file.clone(),
            config,
        )
        .await
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers(Any);

    // Build the router
    let app = Router::new()
        .without_v07_checks() // Allow compatibility with axum 0.7 types from IC crates
        // Metrics endpoint
        .route(
            "/metrics",
            get(|| async {
                let mut buffer = Vec::new();
                let encoder = TextEncoder::new();
                encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
                String::from_utf8(buffer).unwrap()
            }),
        )
        // Health check
        .route("/health", get(|| async { "OK" }))
        // Config endpoint (to see current configuration)
        .route("/config", get(vm_handlers::get_config))
        // VM routes
        .route("/vms/list", get(vm_handlers::list_vms))
        .route("/vms/provision", post(vm_handlers::provision_vm))
        .route("/vms/delete", post(vm_handlers::delete_vm))
        // Node routes
        .route("/nodes/list", get(node_handlers::list_nodes))
        .route("/nodes/get", post(node_handlers::get_node))
        // Subnet routes
        .route("/subnets/list", get(subnet_handlers::list_subnets))
        .route("/subnets/create", post(subnet_handlers::create_subnet_proposal))
        .route("/subnets/delete", post(subnet_handlers::delete_subnet_proposal))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Layers
        .layer(cors)
        .layer(metrics_layer)
        .with_state(state.clone());

    // Start the registry sync loop
    let registry_sync_stop = rt.block_on(async { state.start_registry_sync_loop() });

    // Start the server
    let port = cli_args.port;
    let server_handle = rt.spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
        info!(state.log, "Server started on port {}", port);
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                server_stop_rx.await.ok();
            })
            .await
            .unwrap();
        info!(state.log, "Server stopped");
    });

    // Wait for shutdown signal
    rt.block_on(shutdown_signal);

    // Stop registry sync loop
    registry_sync_stop.send(true).ok();

    // Signal server to stop
    server_stop_tx.send(()).ok();

    // Wait for server to stop
    rt.block_on(server_handle).unwrap();

    // Shutdown provider
    provider.shutdown().unwrap();
}

fn make_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).chan_size(8192).build();
    Logger::root(drain.fuse(), o!())
}

/// Create a shutdown signal future that completes on SIGINT or SIGTERM
async fn make_shutdown_signal(log: Logger) {
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
}

#[derive(Parser, Debug)]
#[clap(about = "Cloud Engine Controller Backend", version)]
pub struct CliArgs {
    #[clap(long = "config-file", help = "Path to application config JSON file (GCP project, zones, etc.)")]
    config_file: PathBuf,

    #[clap(long = "targets-dir", help = "Directory for storing registry local store")]
    targets_dir: PathBuf,

    #[clap(long = "port", default_value = "8000", help = "Server port")]
    port: u16,

    #[clap(
        long = "poll-interval",
        default_value = "30s",
        value_parser = parse_duration,
        help = "Registry sync interval"
    )]
    poll_interval: Duration,

    #[clap(
        long = "query-request-timeout",
        default_value = "5s",
        value_parser = parse_duration,
        help = "Registry query timeout"
    )]
    registry_query_timeout: Duration,

    #[clap(long = "nns-url", default_value = "https://ic0.app", help = "NNS URL for registry sync")]
    nns_url: Url,

    #[clap(long = "gcp-credentials-file", help = "Path to GCP service account JSON file (uses ADC if not provided)")]
    gcp_credentials_file: Option<PathBuf>,
}
