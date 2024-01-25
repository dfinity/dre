use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use slog::{info, Logger};
use tokio::sync::Mutex;
use warp::{Filter, Rejection};

use crate::definition::Definition;
use crate::server_handlers::add_boundary_node_to_definition_handler::add_boundary_node;
use crate::server_handlers::add_boundary_node_to_definition_handler::AddBoundaryNodeToDefinitionBinding;
use crate::server_handlers::add_definition_handler::{
    add_definition, AddDefinitionBinding, AddDefinitionBinding as ReplaceDefinitionsBinding,
};
use crate::server_handlers::delete_definition_handler::delete_definition;
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

pub(crate) struct Server {
    log: Logger,
    items: Arc<Mutex<Vec<Definition>>>,
    poll_interval: Duration,
    registry_query_timeout: Duration,
    registry_path: PathBuf,
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    rt: tokio::runtime::Handle,
}

impl Server {
    pub(crate) fn new(
        log: Logger,
        items: Arc<Mutex<Vec<Definition>>>,
        poll_interval: Duration,
        registry_query_timeout: Duration,
        registry_path: PathBuf,
        handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
        rt: tokio::runtime::Handle,
    ) -> Self {
        Self {
            log,
            items,
            poll_interval,
            registry_query_timeout,
            registry_path,
            handles,
            rt,
        }
    }
    pub(crate) async fn run(self, recv: tokio::sync::oneshot::Receiver<()>) {
        let poll_interval = self.poll_interval;
        let registry_query_timeout = self.registry_query_timeout;

        let add_items = self.items.clone();
        let add_log = self.log.clone();
        let add_handles = self.handles.clone();
        let add_rt = self.rt.clone();
        let add_registry_path = self.registry_path.clone();
        let add = warp::path::end()
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || AddDefinitionBinding {
                definitions: add_items.clone(),
                log: add_log.clone(),
                poll_interval,
                registry_query_timeout,
                registry_path: add_registry_path.clone(),
                handles: add_handles.clone(),
                rt: add_rt.clone(),
            }))
            .and_then(add_definition);

        let put_items = self.items.clone();
        let put_log = self.log.clone();
        let put_handles = self.handles.clone();
        let put_rt = self.rt.clone();
        let put_registry_path = self.registry_path.clone();
        let put = warp::path::end()
            .and(warp::put())
            .and(warp::body::json())
            .and(warp::any().map(move || ReplaceDefinitionsBinding {
                definitions: put_items.clone(),
                log: put_log.clone(),
                poll_interval,
                registry_query_timeout,
                registry_path: put_registry_path.clone(),
                handles: put_handles.clone(),
                rt: put_rt.clone(),
            }))
            .and_then(replace_definitions);

        let get_items = self.items.clone();
        let get = warp::path::end()
            .and(warp::get())
            .and(warp::any().map(move || get_items.clone()))
            .and_then(get_definitions);

        let delete_items = self.items.clone();
        let delete = warp::path!(String)
            .and(warp::delete())
            .and(warp::any().map(move || delete_items.clone()))
            .and_then(delete_definition);

        let export_items = self.items.clone();
        let export_def_log = self.log.clone();
        let export_prometheus = warp::path!("prom" / "targets")
            .and(warp::get())
            .and(warp::any().map(move || ExportDefinitionConfigBinding {
                definitions: export_items.clone(),
                log: export_def_log.clone(),
            }))
            .and_then(export_prometheus_config);

        let export_targets_items = self.items.clone();
        let export_log = self.log.clone();
        let export_targets = warp::path!("targets")
            .and(warp::get())
            .and(warp::any().map(move || ExportTargetsBinding {
                definitions: export_targets_items.clone(),
                log: export_log.clone(),
            }))
            .and_then(export_targets);

        let add_boundary_node_targets = self.items.clone();
        let add_boundary_node_log = self.log.clone();
        let add_boundary_node = warp::path!("add_boundary_node")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || AddBoundaryNodeToDefinitionBinding {
                definitions: add_boundary_node_targets.clone(),
                log: add_boundary_node_log.clone(),
            }))
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
