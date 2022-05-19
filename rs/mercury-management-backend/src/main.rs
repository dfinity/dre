mod backend_type;
mod endpoints;
mod prom;
mod proposal;
mod registry;
mod release;
use actix_web::dev::Service;
use actix_web::{error, get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
mod gitlab;
mod health;
use crate::prom::{ICProm, PromClient};
use ::gitlab::{AsyncGitlab, GitlabBuilder};
use ic_protobuf::registry::crypto::v1::PublicKey;
use ic_registry_common::registry::RegistryCanister;
use ic_registry_keys::{make_crypto_threshold_signing_pubkey_key, ROOT_SUBNET_ID_KEY};
use ic_types::{crypto::threshold_sig::ThresholdSigPublicKey, PrincipalId, SubnetId};
use log::{info, warn};
use mercury_management_types::{Location, ProviderDetails};
use registry_canister::mutations::common::decode_registry_value;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use url::Url;
extern crate env_logger;
use ic_interfaces::registry::RegistryValue;

const GITLAB_TOKEN_ENV: &str = "GITLAB_API_TOKEN";
const GITLAB_TOKEN_IC_PUBLIC_ENV: &str = "GITLAB_API_TOKEN_IC_PUBLIC";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let registry_state = Arc::new(RwLock::new(registry::RegistryState::new(
        gitlab_client(GITLAB_TOKEN_ENV).await.into(),
        gitlab_client(GITLAB_TOKEN_IC_PUBLIC_ENV).await.into(),
    )));
    let registry_state_poll = registry_state.clone();
    let prom_client = Arc::new(
        PromClient::new("prometheus.dfinity.systems:9090", None).expect("Couldn't initialize prometheus client"),
    );

    tokio::spawn(async { poll(registry_state_poll).await });

    HttpServer::new(move || {
        let middleware_registry_state = registry_state.clone();
        App::new()
            .app_data(web::Data::new(registry_state.clone()))
            .app_data(web::Data::new(prom_client.clone()))
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
            .service(version)
            .service(subnets)
            .service(nodes)
            .service(missing_hosts)
            .service(hosts)
            .service(removed_nodes)
            .service(operators)
            .service(nodes_healths)
            .service(ic_single_metrics)
            .service(ic_agg_metrics)
            .service(subnet_healths)
            .service(get_subnet)
            .service(endpoints::subnet::pending_action)
            .service(endpoints::subnet::replace)
    })
    .bind(("0.0.0.0", 8080))?
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

#[get("/subnet_healths")]
async fn subnet_healths(
    actix_web::web::Query(subnet): actix_web::web::Query<SubnetRequest>,
    registry_state: web::Data<Arc<RwLock<registry::RegistryState>>>,
) -> impl Responder {
    let principal = match PrincipalId::from_str(&subnet.id) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().json(e.to_string()),
    };
    let full_subnet = registry_state
        .read()
        .await
        .subnets()
        .get(&principal)
        .expect("No subnet with that ID")
        .clone();
    let out = match prom::node_healths_per_subnet(full_subnet).await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    };
    out
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
    let update_version_proposals = proposal::ProposalAgent::new()
        .list_update_subnet_version_proposals()
        .await
        .map_err(error::ErrorInternalServerError)?;
    let registry = registry.read().await;
    let nodes_versions = prom::subnets_upgraded(registry.subnets(), update_version_proposals.clone())
        .await
        .map_err(error::ErrorInternalServerError)?;
    response_from_result(release::Rollout::new(
        registry.subnets(),
        update_version_proposals,
        nodes_versions,
        registry.replica_releases(),
    ))
}

#[get("/version")]
async fn version(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.version()).await
}

#[get("/subnets")]
async fn subnets(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.subnets()).await
}

#[get("/nodes")]
async fn nodes(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(registry.nodes_with_proposals().await)
}

#[get("/nodes/healths")]
async fn nodes_healths(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> Result<HttpResponse, Error> {
    let registry = registry.read().await;
    response_from_result(health::nodes().await.map(|mut healths| {
        registry
            .nodes()
            .values()
            .map(|n| {
                (
                    n.principal,
                    healths.remove(&n.principal).unwrap_or(health::Status::Unknown),
                )
            })
            .collect::<HashMap<_, _>>()
    }))
}

#[get("/missing_hosts")]
async fn missing_hosts(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.missing_hosts()).await
}

#[get("/hosts")]
async fn hosts(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.hosts()).await
}

#[get("/operators")]
async fn operators(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.operators()).await
}

#[get("/removed_nodes")]
async fn removed_nodes(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.removed_nodes()).await
}

async fn query_registry<T: Serialize>(
    registry: web::Data<Arc<RwLock<registry::RegistryState>>>,
    query: fn(&registry::RegistryState) -> T,
) -> actix_web::HttpResponse {
    HttpResponse::Ok().json(query(registry.clone().read().await.deref()))
}

async fn nns_public_key(registry_canister: &RegistryCanister) -> anyhow::Result<ThresholdSigPublicKey> {
    let (nns_subnet_id_vec, _) = registry_canister
        .get_value(ROOT_SUBNET_ID_KEY.as_bytes().to_vec(), None)
        .await
        .map_err(|e| anyhow::format_err!("failed to get root subnet: {}", e))?;
    let nns_subnet_id = decode_registry_value::<ic_protobuf::types::v1::SubnetId>(nns_subnet_id_vec);
    let (nns_pub_key_vec, _) = registry_canister
        .get_value(
            make_crypto_threshold_signing_pubkey_key(SubnetId::new(
                PrincipalId::try_from(nns_subnet_id.principal_id.unwrap().raw).unwrap(),
            ))
            .as_bytes()
            .to_vec(),
            None,
        )
        .await
        .map_err(|e| anyhow::format_err!("failed to get public key: {}", e))?;
    Ok(
        ThresholdSigPublicKey::try_from(PublicKey::decode(nns_pub_key_vec.as_slice()).expect("invalid public key"))
            .expect("failed to create thresholdsig public key"),
    )
}

fn nns_nodes_urls() -> Vec<Url> {
    vec![Url::parse("https://nns.ic0.app").unwrap()]
}

async fn poll(registry_state: Arc<RwLock<registry::RegistryState>>) {
    let registry_canister = RegistryCanister::new(nns_nodes_urls());
    let mut nns_pub_key = nns_public_key(&registry_canister).await;
    let mut backoff_seconds = 1;
    while let Err(e) = nns_pub_key {
        warn!("failed to fetch NNS public key: {}", e);
        nns_pub_key = nns_public_key(&registry_canister).await;
        sleep(Duration::from_secs(backoff_seconds)).await;
        backoff_seconds = std::cmp::min(backoff_seconds * 2, 60);
    }
    let nns_pub_key = nns_pub_key.unwrap();

    loop {
        // Value extracted here to avoid deadlock
        let registry_state_version = registry_state.read().await.version();
        match registry_canister
            .get_certified_changes_since(registry_state_version, &nns_pub_key)
            .await
        {
            Ok((deltas, ..)) => {
                if !deltas.is_empty() {
                    let locations_result = query_ic_dashboard_list::<Vec<Location>>("v2/locations").await;
                    let providers_result = query_ic_dashboard_list::<Vec<ProviderDetails>>("node-providers/list").await;

                    match locations_result
                        .and_then(|locations| providers_result.map(|providers| (locations, providers)))
                    {
                        Ok((locations, providers)) => {
                            let mut registry_state = registry_state.write().await;
                            match registry_state.update(deltas, locations, providers).await {
                                Ok(_) => info!(
                                    "Finished the update loop, reached registry version {}",
                                    registry_state.version()
                                ),
                                Err(e) => warn!("failed state update: {}", e),
                            }
                        }
                        Err(e) => {
                            warn!("Failed querying IC dashboard {}", e);
                        }
                    }
                }
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(e) => {
                warn!("Poll failed: {}", e);
            }
        }
    }
}

async fn query_ic_dashboard_list<T: DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let client = reqwest::Client::new();
    let result = client
        .get(format!("https://ic-api.internetcomputer.org/api/{}", &path))
        .send()
        .await
        .and_then(|r| r.error_for_status());
    match result {
        Ok(response) => match response.json::<T>().await {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow::format_err!("failed to parse response: {}", e)),
        },
        Err(e) => Err(anyhow::format_err!("failed to fetch response: {}", e)),
    }
}

#[post("/metrics")]
async fn ic_single_metrics(
    prom: web::Data<Arc<PromClient>>,
    params: web::Json<backend_type::ICNetworkQuerySingle>,
) -> Result<HttpResponse, Error> {
    let resp: Result<serde_json::Value, anyhow::Error> = prom.matching_single_query_call(params.into_inner()).await;
    match resp {
        Ok(v) => Ok(HttpResponse::Ok().json(v)),
        Err(_e) => Err(Error::from(prom::TextError {
            name: "Prometheus client error, check params",
        })),
    }
}

#[post("/aggregated_matrics")]
async fn ic_agg_metrics(
    prom: web::Data<Arc<PromClient>>,
    params: web::Json<backend_type::ICNetworkQueryAggregate>,
) -> Result<HttpResponse, Error> {
    let resp = prom.matching_aggregate_query_call(params.into_inner()).await;
    match resp {
        Ok(v) => Ok(HttpResponse::Ok().json(v)),
        Err(_e) => Err(Error::from(prom::TextError {
            name: "Prometheus client error, check params",
        })),
    }
}

fn response_from_result<T: Serialize, E: std::fmt::Debug + std::fmt::Display + 'static>(
    result: Result<T, E>,
) -> Result<HttpResponse, Error> {
    match result {
        Ok(data) => Ok(HttpResponse::Ok().json(data)),
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}

async fn gitlab_client(env: &str) -> AsyncGitlab {
    GitlabBuilder::new(
        "gitlab.com",
        std::env::var(env).unwrap_or_else(|_| panic!("missing {} env variable", env)),
    )
    .build_async()
    .await
    .expect("unable to initialize gitlab token")
}
