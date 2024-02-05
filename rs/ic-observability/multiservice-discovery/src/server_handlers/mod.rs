use std::path::PathBuf;
use std::time::Duration;

use axum::routing::{delete, get, post, put};
use axum::Router;
use slog::{info, Logger};

use crate::definition::DefinitionsSupervisor;
use crate::server_handlers::add_boundary_node_to_definition_handler::add_boundary_node;
use crate::server_handlers::add_definition_handler::add_definition;
use crate::server_handlers::delete_definition_handler::delete_definition;
use crate::server_handlers::export_prometheus_config_handler::export_prometheus_config;
use crate::server_handlers::export_targets_handler::export_targets;
use crate::server_handlers::get_definition_handler::get_definitions;
use crate::server_handlers::replace_definitions_handler::replace_definitions;

mod add_boundary_node_to_definition_handler;
mod add_definition_handler;
mod delete_definition_handler;
pub mod dto;
pub mod export_prometheus_config_handler;
mod export_targets_handler;
mod get_definition_handler;
mod replace_definitions_handler;

// pub type WebResult<T> = Result<T, Rejection>;

// pub(crate) fn ok(log: Logger, message: String) -> WebResult<warp::reply::WithStatus<String>> {
//     info!(log, "{}", message);
//     let r: WithStatus<String> = warp::reply::with_status(message, warp::http::StatusCode::OK);
//     let rr: WebResult<warp::reply::WithStatus<String>> = Ok(r);
//     rr
// }

// pub(crate) fn bad_request<T>(log: Logger, message: String, err: T) -> WebResult<warp::reply::WithStatus<String>>
// where
//     T: Display,
// {
//     info!(log, "{}: {}", message, err);
//     Ok(warp::reply::with_status(
//         format!("{}: {}", message, err),
//         warp::http::StatusCode::BAD_REQUEST,
//     ))
// }

// pub(crate) fn not_found<T>(log: Logger, message: String, err: T) -> WebResult<warp::reply::WithStatus<String>>
// where
//     T: Display,
// {
//     info!(log, "{}: {}", message, err);
//     Ok(warp::reply::with_status(
//         format!("{}: {}", message, err),
//         warp::http::StatusCode::NOT_FOUND,
//     ))
// }
// pub(crate) fn forbidden<T>(log: Logger, message: String, err: T) -> WebResult<warp::reply::WithStatus<String>>
// where
//     T: Display,
// {
//     info!(log, "{}: {}", message, err);
//     Ok(warp::reply::with_status(
//         format!("{}: {}", message, err),
//         warp::http::StatusCode::FORBIDDEN,
//     ))
// }

#[derive(Clone)]
pub(crate) struct Server {
    log: Logger,
    supervisor: DefinitionsSupervisor,
    poll_interval: Duration,
    registry_query_timeout: Duration,
    registry_path: PathBuf,
}

impl Server {
    pub(crate) fn new(
        log: Logger,
        supervisor: DefinitionsSupervisor,
        poll_interval: Duration,
        registry_query_timeout: Duration,
        registry_path: PathBuf,
    ) -> Self {
        Self {
            log,
            supervisor,
            poll_interval,
            registry_query_timeout,
            registry_path,
        }
    }
    pub(crate) async fn run(self, recv: tokio::sync::oneshot::Receiver<()>) {
        let app = Router::new()
            .route("/", post(add_definition))
            .route("/", put(replace_definitions))
            .route("/", get(get_definitions))
            .route("/:name", delete(delete_definition))
            .route("/prom/targets", get(export_prometheus_config))
            .route("/targets", get(export_targets))
            .route("/add_boundary_node", post(add_boundary_node))
            .with_state(self.clone());

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
        info!(self.log, "Server started on port {}", 8000);
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                recv.await.unwrap();
            })
            .await
            .unwrap();
        info!(self.log, "Server stopped");
    }
}
