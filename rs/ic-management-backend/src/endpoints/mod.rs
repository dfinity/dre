pub mod governance_canister;
pub mod nodes_ops;
pub mod query_decentralization;
pub mod release;
pub mod subnet;

use crate::{
    config::get_nns_url_vec_from_target_network, gitlab_dfinity, health, prometheus, proposal, registry,
    registry::RegistryState, release::list_subnets_release_statuses, release::RolloutBuilder,
};
use actix_web::dev::Service;
use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer, Responder, Result};
use decentralization::network::AvailableNodesQuerier;
use ic_management_types::Network;
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_types::PrincipalId;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::process;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const GITLAB_TOKEN_RELEASE_ENV: &str = "GITLAB_API_TOKEN_RELEASE";
const GITLAB_API_TOKEN_FALLBACK: &str = "GITLAB_API_TOKEN";

pub async fn run_backend(
    target_network: Network,
    listen_ip: &str,
    listen_port: u16,
    run_from_release_cli: bool,
    mpsc_tx: Option<std::sync::mpsc::Sender<actix_web::dev::ServerHandle>>,
) -> std::io::Result<()> {
    debug!("Starting backend");
    let registry_state = Arc::new(RwLock::new(
        registry::RegistryState::new(target_network.clone(), run_from_release_cli).await,
    ));

    if run_from_release_cli {
        registry::update_node_details(&registry_state).await;
    } else {
        if std::env::var(GITLAB_TOKEN_RELEASE_ENV).is_err() {
            let fallback_token = std::env::var(GITLAB_API_TOKEN_FALLBACK);
            if fallback_token.is_err() {
                error!(
                    "Could not lead the Gitlab token from variable {} or {}",
                    GITLAB_TOKEN_RELEASE_ENV, GITLAB_API_TOKEN_FALLBACK
                );
                process::exit(exitcode::CONFIG);
            }
            std::env::set_var(GITLAB_TOKEN_RELEASE_ENV, fallback_token.unwrap());
        }
        let gitlab_client_release_repo = gitlab_dfinity::authenticated_client(GITLAB_TOKEN_RELEASE_ENV).await;
        let closure_target_network = target_network.clone();
        let registry_state_poll = registry_state.clone();
        tokio::spawn(async {
            registry::poll(gitlab_client_release_repo, registry_state_poll, closure_target_network).await
        });
    }

    let num_workers = if run_from_release_cli { 1 } else { 8 };

    let mut srv = HttpServer::new(move || {
        let network = target_network.clone();
        // For release_cli invocations we don't need more than one worker

        let middleware_registry_state = registry_state.clone();
        App::new()
            .app_data(web::Data::new(registry_state.clone()))
            .wrap_fn(move |req, srv| {
                let fut = srv.call(req);
                let registry_state = middleware_registry_state.clone();
                let network = network.clone();
                async move {
                    let nns_urls = get_nns_url_vec_from_target_network(&network);
                    let registry_canister = RegistryCanister::new(nns_urls.clone());
                    let registry_reader = registry_state.read().await;
                    let registry_version = registry_reader.version();
                    if registry_canister
                        .get_latest_version()
                        .await
                        .map_or(true, |v| v != registry_version && !run_from_release_cli)
                    {
                        Err(actix_web::error::ErrorServiceUnavailable("version updating"))
                    } else {
                        let res = fut.await?;
                        Ok(res)
                    }
                }
            })
            .service(rollout)
            .service(subnets_release)
            .service(version)
            .service(list_subnets)
            .service(nodes)
            .service(available_nodes)
            .service(missing_guests)
            .service(guests)
            .service(operators)
            .service(nodes_healths)
            .service(get_subnet)
            .service(self::subnet::pending_action)
            .service(self::subnet::replace)
            .service(self::subnet::create_subnet)
            .service(self::subnet::resize)
            .service(self::subnet::change_preview)
            .service(self::nodes_ops::remove)
            .service(self::query_decentralization::decentralization_subnet_query)
            .service(self::query_decentralization::decentralization_whatif_query)
            .service(self::release::releases_list_all)
            .service(self::release::retireable)
            .service(self::release::get_nns_replica_version)
            .service(self::governance_canister::governance_canister_version_endpoint)
    })
    .shutdown_timeout(10)
    .workers(num_workers)
    .bind((listen_ip, listen_port))
    .unwrap();

    if run_from_release_cli {
        // params reference: https://github.com/actix/actix-web/blob/master/actix-web/tests/test_httpserver.rs
        srv = srv
            .backlog(1)
            .max_connections(10)
            .max_connection_rate(10)
            .client_request_timeout(Duration::from_secs(60))
            .client_disconnect_timeout(Duration::ZERO)
            .server_hostname("localhost")
            .system_exit();
    }

    let srv = srv.run();

    info!("Backend started at {}:{}", listen_ip, listen_port);

    if let Some(mpsc_tx) = mpsc_tx {
        mpsc_tx.send(srv.handle()).unwrap();
    }

    srv.await
}

#[derive(Deserialize, Serialize)]
pub struct SubnetRequest {
    id: String,
}

#[derive(Deserialize, Serialize)]
pub struct NewSubnet {
    size: i32,
    exclusions: Option<Vec<PrincipalId>>,
}

#[get("/subnet")]
async fn get_subnet(
    registry: web::Data<Arc<RwLock<registry::RegistryState>>>,
    web::Query(subnet): actix_web::web::Query<SubnetRequest>,
) -> impl Responder {
    let principal = match PrincipalId::from_str(&subnet.id) {
        Ok(v) => v,
        Err(_e) => {
            return HttpResponse::BadRequest().json("Subnet ID is not a valid principal");
        }
    };
    let subnets_lock = registry.read().await.subnets();
    let maybe_record = subnets_lock.get(&principal);
    let record = match maybe_record {
        Some(v) => v.clone(),
        None => {
            return HttpResponse::BadRequest().json("Subnet ID does not exist");
        }
    };
    HttpResponse::Ok().json(record)
}

#[get("/rollout")]
async fn rollout(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let proposal_agent = proposal::ProposalAgent::new(registry.nns_url());
    let network = registry.network();
    let prometheus_client = prometheus::client(&network);
    let service = RolloutBuilder {
        proposal_agent,
        prometheus_client,
        subnets: registry.subnets(),
        releases: registry.replica_releases(),
        network,
    };
    response_from_result(service.build().await)
}

#[get("/subnets/versions")]
async fn subnets_release(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let proposal_agent = proposal::ProposalAgent::new(registry.nns_url());
    let network = registry.network();
    let prometheus_client = prometheus::client(&network);
    response_from_result(
        list_subnets_release_statuses(
            &proposal_agent,
            &prometheus_client,
            network,
            registry.subnets(),
            registry.replica_releases(),
        )
        .await,
    )
}

#[get("/version")]
async fn version(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.version()).await
}

#[get("/subnets")]
async fn list_subnets(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    let registry = registry.read().await;
    response_from_result(registry.subnets_with_proposals().await)
}

#[get("/nodes")]
async fn nodes(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.nodes_with_proposals().await)
}

#[get("/nodes/available")]
async fn available_nodes(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.available_nodes().await)
}

#[get("/nodes/healths")]
async fn nodes_healths(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    let health_client = health::HealthClient::new(registry.network());
    response_from_result(health_client.nodes().await.map(|mut healths| {
        registry
            .nodes()
            .values()
            .map(|n| {
                (
                    n.principal,
                    healths
                        .remove(&n.principal)
                        .unwrap_or(ic_management_types::Status::Unknown),
                )
            })
            .collect::<BTreeMap<_, _>>()
    }))
}

#[get("/missing_guests")]
async fn missing_guests(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.missing_guests()).await
}

#[get("/guests")]
async fn guests(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.guests()).await
}

#[get("/operators")]
async fn operators(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.operators()).await
}

fn response_from_result<T: Serialize, E: std::fmt::Debug + std::fmt::Display + 'static>(
    result: Result<T, E>,
) -> Result<HttpResponse, Error> {
    match result {
        Ok(data) => Ok(HttpResponse::Ok().json(data)),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

async fn query_registry<T: Serialize>(
    registry: web::Data<Arc<RwLock<registry::RegistryState>>>,
    query: fn(&registry::RegistryState) -> T,
) -> actix_web::HttpResponse {
    HttpResponse::Ok().json(query(registry.clone().read().await.deref()))
}
