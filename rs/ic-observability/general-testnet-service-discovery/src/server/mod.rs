use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use axum_otel_metrics::HttpMetricsLayer;
use slog::{Logger, info};
use tokio_util::sync::CancellationToken;

use crate::supervisor::TargetSupervisor;

mod add_targets;
mod get_targets;

pub type WebResult<T> = Result<T, (StatusCode, String)>;

#[derive(Clone)]
pub struct Server {
    logger: Logger,
    token: CancellationToken,
    port: u16,
}

impl Server {
    pub fn new(logger: Logger, token: CancellationToken, port: u16) -> Self {
        Self { logger, token, port }
    }

    pub async fn run(self, metrics_layer: HttpMetricsLayer, supervisor: TargetSupervisor) {
        let app = Router::new()
            .without_v07_checks()
            .route("/", get(get_targets::get_targets))
            .route("/", post(add_targets::add_targets))
            .layer(metrics_layer)
            .with_state(supervisor);

        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), self.port);
        let listener = tokio::net::TcpListener::bind(socket).await.unwrap();
        info!(self.logger, "Server started on port {}", self.port);
        let logger_clone = self.logger.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                self.token.cancelled().await;
                info!(logger_clone, "Received shutdown in server loop");
            })
            .await
            .unwrap();
        info!(self.logger, "Server stopped");
    }
}
