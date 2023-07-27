mod config;
mod endpoints;
mod factsdb;
mod gitlab;
mod health;
mod prometheus;
mod proposal;
mod public_dashboard;
mod registry;
mod release;
use crate::release::RolloutBuilder;
use actix_web::dev::Service;
use actix_web::{error, get, web, App, Error, HttpResponse, HttpServer, Responder};
use config::{nns_nodes_urls, nns_url};
use decentralization::network::AvailableNodesQuerier;
use dotenv::dotenv;
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_types::PrincipalId;
use log::{error, info};
use registry::{local_registry_path, sync_local_store};
use release::list_subnets_release_statuses;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
extern crate env_logger;
use ic_registry_local_registry::LocalRegistry;

const GITLAB_TOKEN_IC_PUBLIC_ENV: &str = "GITLAB_API_TOKEN_IC_PUBLIC";
const GITLAB_TOKEN_RELEASE_ENV: &str = "GITLAB_API_TOKEN_RELEASE";
const GITLAB_API_TOKEN_FALLBACK: &str = "GITLAB_API_TOKEN";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let target_network = config::target_network();
    sync_local_store(target_network.clone())
        .await
        .expect("failed to init local store");

    let local_registry_path = local_registry_path(target_network.clone());
    let local_registry: Arc<LocalRegistry> = Arc::new(
        LocalRegistry::new(local_registry_path, Duration::from_millis(1000)).expect("Failed to create local registry"),
    );

    tokio::spawn(async move {
        let mut print_counter = 0;
        loop {
            let print_enabled = print_counter % 10 == 0;
            if print_enabled {
                info!("Updating local registry");
            }
            if let Err(e) = sync_local_store(config::target_network()).await {
                error!("Failed to update local registry: {}", e);
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            print_counter += 1;
        }
    });

    if std::env::var(GITLAB_TOKEN_RELEASE_ENV).is_err() {
        std::env::set_var(
            GITLAB_TOKEN_RELEASE_ENV,
            std::env::var(GITLAB_API_TOKEN_FALLBACK).unwrap(),
        );
    }
    if std::env::var(GITLAB_TOKEN_IC_PUBLIC_ENV).is_err() {
        std::env::set_var(
            GITLAB_TOKEN_IC_PUBLIC_ENV,
            std::env::var(GITLAB_API_TOKEN_FALLBACK).unwrap(),
        );
    }
    let registry_state = Arc::new(RwLock::new(registry::RegistryState::new(
        nns_url(),
        target_network,
        local_registry,
        gitlab::authenticated_client(GITLAB_TOKEN_IC_PUBLIC_ENV).await.into(),
    )));
    let registry_state_poll = registry_state.clone();

    let release_repo_gitlab_client = gitlab::authenticated_client(GITLAB_TOKEN_RELEASE_ENV).await;
    tokio::spawn(async { registry::poll(release_repo_gitlab_client, registry_state_poll).await });

    HttpServer::new(move || {
        let middleware_registry_state = registry_state.clone();
        App::new()
            .app_data(web::Data::new(registry_state.clone()))
            .wrap_fn(move |req, srv| {
                let fut = srv.call(req);
                let registry = middleware_registry_state.clone();
                async move {
                    let registry_canister = RegistryCanister::new(nns_nodes_urls());
                    let registry = registry.read().await;
                    let registry_version = registry.version();
                    if registry_canister
                        .get_latest_version()
                        .await
                        .map_or(true, |v| v != registry_version)
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
            .service(endpoints::subnet::pending_action)
            .service(endpoints::subnet::replace)
            .service(endpoints::subnet::create_subnet)
            .service(endpoints::subnet::resize)
            .service(endpoints::subnet::change_preview)
            .service(endpoints::nodes::remove)
            .service(endpoints::query_decentralization::decentralization_subnet_query)
            .service(endpoints::query_decentralization::decentralization_whatif_query)
            .service(endpoints::release::retireable)
            .service(endpoints::release::get_nns_replica_version)
            .service(endpoints::governance_canister::governance_canister_version_endpoint)
    })
    .shutdown_timeout(10)
    .bind((
        "0.0.0.0",
        std::env::var("BACKEND_PORT")
            .map(|p| {
                p.parse()
                    .expect("Unable to parse BACKEND_PORT environment variable as a valid port")
            })
            .unwrap_or(8080),
    ))?
    .run()
    .await
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
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}

async fn query_registry<T: Serialize>(
    registry: web::Data<Arc<RwLock<registry::RegistryState>>>,
    query: fn(&registry::RegistryState) -> T,
) -> actix_web::HttpResponse {
    HttpResponse::Ok().json(query(registry.clone().read().await.deref()))
}
