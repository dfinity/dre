use std::fmt::Display;
use std::path::PathBuf;
use std::time::Duration;

use slog::{info, Logger};
use warp::reply::WithStatus;
use warp::{Filter, Rejection};

use crate::definition::DefinitionsSupervisor;
use crate::server_handlers::add_boundary_node_to_definition_handler::add_boundary_node;
use crate::server_handlers::add_boundary_node_to_definition_handler::AddBoundaryNodeToDefinitionBinding;
use crate::server_handlers::add_definition_handler::{
    add_definition, AddDefinitionBinding, AddDefinitionBinding as ReplaceDefinitionsBinding,
};
use crate::server_handlers::delete_definition_handler::{delete_definition, DeleteDefinitionBinding};
use crate::server_handlers::export_prometheus_config_handler::{
    export_prometheus_config, ExportDefinitionConfigBinding,
};
use crate::server_handlers::export_targets_handler::export_targets;
use crate::server_handlers::export_targets_handler::ExportTargetsBinding;
use crate::server_handlers::get_definition_handler::get_definitions;
use crate::server_handlers::replace_definitions_handler::replace_definitions;

mod add_boundary_node_to_definition_handler;
mod add_definition_handler;
mod delete_definition_handler;
pub mod dto;
mod export_prometheus_config_handler;
mod export_targets_handler;
mod get_definition_handler;
mod replace_definitions_handler;

pub type WebResult<T> = Result<T, Rejection>;

pub(crate) fn ok(log: Logger, message: String) -> WebResult<warp::reply::WithStatus<String>> {
    info!(log, "{}", message);
    let r: WithStatus<String> = warp::reply::with_status(message, warp::http::StatusCode::OK);
    let rr: WebResult<warp::reply::WithStatus<String>> = Ok(r);
    rr
}

pub(crate) fn bad_request<T>(log: Logger, message: String, err: T) -> WebResult<warp::reply::WithStatus<String>>
where
    T: Display,
{
    info!(log, "{}: {}", message, err);
    Ok(warp::reply::with_status(
        format!("{}: {}", message, err),
        warp::http::StatusCode::BAD_REQUEST,
    ))
}

pub(crate) fn not_found<T>(log: Logger, message: String, err: T) -> WebResult<warp::reply::WithStatus<String>>
where
    T: Display,
{
    info!(log, "{}: {}", message, err);
    Ok(warp::reply::with_status(
        format!("{}: {}", message, err),
        warp::http::StatusCode::NOT_FOUND,
    ))
}

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
        let poll_interval = self.poll_interval;
        let registry_query_timeout = self.registry_query_timeout;

        let binding = AddDefinitionBinding {
            supervisor: self.supervisor.clone(),
            log: self.log.clone(),
            poll_interval,
            registry_query_timeout,
            registry_path: self.registry_path.clone(),
        };
        let add = warp::path::end()
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || binding.clone()))
            .and_then(add_definition);

        let binding = ReplaceDefinitionsBinding {
            supervisor: self.supervisor.clone(),
            log: self.log.clone(),
            poll_interval,
            registry_query_timeout,
            registry_path: self.registry_path.clone(),
        };
        let put = warp::path::end()
            .and(warp::put())
            .and(warp::body::json())
            .and(warp::any().map(move || binding.clone()))
            .and_then(replace_definitions);

        let get_items = self.supervisor.clone();
        let get = warp::path::end()
            .and(warp::get())
            .and(warp::any().map(move || get_items.clone()))
            .and_then(get_definitions);

        let binding = DeleteDefinitionBinding {
            supervisor: self.supervisor.clone(),
            log: self.log.clone(),
        };
        let delete = warp::path!(String)
            .and(warp::delete())
            .and(warp::any().map(move || binding.clone()))
            .and_then(delete_definition);

        let binding = ExportDefinitionConfigBinding {
            supervisor: self.supervisor.clone(),
            log: self.log.clone(),
        };
        let export_prometheus = warp::path!("prom" / "targets")
            .and(warp::get())
            .and(warp::any().map(move || binding.clone()))
            .and_then(export_prometheus_config);

        let binding = ExportTargetsBinding {
            supervisor: self.supervisor.clone(),
            log: self.log.clone(),
        };
        let export_targets = warp::path!("targets")
            .and(warp::get())
            .and(warp::any().map(move || binding.clone()))
            .and_then(export_targets);

        let binding = AddBoundaryNodeToDefinitionBinding {
            supervisor: self.supervisor.clone(),
            log: self.log.clone(),
        };
        let add_boundary_node = warp::path!("add_boundary_node")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || binding.clone()))
            .and_then(add_boundary_node);

        let routes = add
            .or(get)
            .or(delete)
            .or(put)
            .or(export_prometheus)
            .or(export_targets)
            .or(add_boundary_node);

        let routes = routes.with(warp::log("multiservice_discovery"));
        let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], 8000), async {
            recv.await.ok();
        });
        info!(self.log, "Server started on port {}", 8000);
        server.await;
        info!(self.log, "Server stopped");
    }
}
