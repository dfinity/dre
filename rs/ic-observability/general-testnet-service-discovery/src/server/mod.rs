use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};
use axum_otel_metrics::HttpMetricsLayer;
use slog::{info, Logger};
use tokio_util::sync::CancellationToken;

use crate::supervisor::TargetSupervisor;

mod add_targets;
mod get_targets;

pub type WebResult<T> = Result<T, (StatusCode, String)>;

#[derive(Clone)]
pub struct Server {
    logger: Logger,
    token: CancellationToken,
}

impl Server {
    pub fn new(logger: Logger, token: CancellationToken) -> Self {
        Self { logger, token }
    }

    pub async fn run(self, metrics_layer: HttpMetricsLayer, supervisor: TargetSupervisor) {
        let app = Router::new()
            .merge(metrics_layer.routes())
            .route("/", get(get_targets::get_targets))
            .route("/", post(add_targets::add_targets))
            .layer(metrics_layer)
            .with_state(supervisor);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
        info!(self.logger, "Server started on port {}", 8000);
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
