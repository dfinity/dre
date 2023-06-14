mod endpoints;
mod prometheus;
mod proposal;
mod registry;
mod release;
use ::gitlab::api::AsyncQuery;
use actix_web::dev::Service;
use actix_web::{error, get, web, App, Error, HttpResponse, HttpServer, Responder};
use decentralization::network::AvailableNodesQuerier;
use dotenv::dotenv;
use hyper::StatusCode;
use ic_base_types::{RegistryVersion, SubnetId};
use ic_protobuf::registry::crypto::v1::PublicKey;
use ic_registry_client::client::ThresholdSigPublicKey;
use ic_registry_keys::{make_crypto_threshold_signing_pubkey_key, ROOT_SUBNET_ID_KEY};
use ic_registry_local_store::{Changelog, ChangelogEntry, KeyMutation, LocalStoreImpl, LocalStoreWriter};
use registry_canister::mutations::common::decode_registry_value;
mod gitlab;
mod health;
use crate::release::RolloutBuilder;
use ::gitlab::{AsyncGitlab, GitlabBuilder};
use futures::TryFutureExt;
use ic_interfaces_registry::{RegistryClient, RegistryValue, ZERO_REGISTRY_VERSION};
use ic_management_types::{FactsDBGuest, Guest, Network, NodeProvidersResponse};
use ic_registry_client_fake::FakeRegistryClient;
use ic_registry_common_proto::pb::local_store::v1::{
    ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType,
};
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_types::PrincipalId;
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ops::{Add, Deref};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use url::Url;
extern crate env_logger;

use ic_registry_local_registry::LocalRegistry;

const GITLAB_TOKEN_IC_PUBLIC_ENV: &str = "GITLAB_API_TOKEN_IC_PUBLIC";
const GITLAB_TOKEN_RELEASE_ENV: &str = "GITLAB_API_TOKEN_RELEASE";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    sync_local_store().await.expect("failed to init local store");

    let local_registry_path = local_registry_path();
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
            if let Err(e) = sync_local_store().await {
                error!("Failed to update local registry: {}", e);
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            print_counter += 1;
        }
    });

    let registry_state = Arc::new(RwLock::new(registry::RegistryState::new(
        nns_url(),
        network(),
        local_registry,
        gitlab_client(GITLAB_TOKEN_IC_PUBLIC_ENV).await.into(),
    )));
    let registry_state_poll = registry_state.clone();

    let release_repo_gitlab_client = gitlab_client(GITLAB_TOKEN_RELEASE_ENV).await;
    tokio::spawn(async { poll(release_repo_gitlab_client, registry_state_poll).await });

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
            .service(version)
            .service(subnets)
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

#[get("/version")]
async fn version(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
    query_registry(registry, |r| r.version()).await
}

#[get("/subnets")]
async fn subnets(registry: web::Data<Arc<RwLock<registry::RegistryState>>>) -> impl Responder {
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

async fn query_registry<T: Serialize>(
    registry: web::Data<Arc<RwLock<registry::RegistryState>>>,
    query: fn(&registry::RegistryState) -> T,
) -> actix_web::HttpResponse {
    HttpResponse::Ok().json(query(registry.clone().read().await.deref()))
}

fn nns_url() -> String {
    std::env::var("NNS_URL").expect("NNS_URL environment variable not provided")
}

fn nns_nodes_urls() -> Vec<Url> {
    vec![Url::parse(&nns_url()).expect("Cannot parse NNS_URL environment variable as a valid URL")]
}

async fn sync_local_store() -> anyhow::Result<()> {
    let local_registry_path = local_registry_path();
    let local_store = Arc::new(LocalStoreImpl::new(local_registry_path.clone()));
    let registry_canister = RegistryCanister::new(nns_nodes_urls());
    let mut latest_version = if !Path::new(&local_registry_path).exists() {
        ZERO_REGISTRY_VERSION
    } else {
        let registry_cache = FakeRegistryClient::new(local_store.clone());
        registry_cache.update_to_latest_version();
        registry_cache.get_latest_version()
    };
    info!("Syncing local registry from version {}", latest_version);
    let mut latest_certified_time = 0;
    let mut updates = vec![];
    let nns_public_key = nns_public_key(&registry_canister).await?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if match registry_canister.get_latest_version().await {
            Ok(v) => {
                info!("Latest registry version: {}", v);
                v == latest_version.get()
            }
            Err(e) => {
                error!("Failed to get latest registry version: {}", e);
                false
            }
        } {
            break;
        }
        if let Ok((mut initial_records, _, t)) = registry_canister
            .get_certified_changes_since(latest_version.get(), &nns_public_key)
            .await
        {
            initial_records.sort_by_key(|tr| tr.version);
            let changelog = initial_records.iter().fold(Changelog::default(), |mut cl, r| {
                let rel_version = (r.version - latest_version).get();
                if cl.len() < rel_version as usize {
                    cl.push(ChangelogEntry::default());
                }
                cl.last_mut().unwrap().push(KeyMutation {
                    key: r.key.clone(),
                    value: r.value.clone(),
                });
                cl
            });

            let versions_count = changelog.len();

            changelog.into_iter().enumerate().for_each(|(i, ce)| {
                let v = RegistryVersion::from(i as u64 + 1 + latest_version.get());
                let local_registry_path = local_registry_path.clone();
                updates.push(async move {
                    let path_str = format!("{:016x}.pb", v.get());
                    // 00 01 02 03 04 / 05 / 06 / 07.pb
                    let v_path = &[
                        &path_str[0..10],
                        &path_str[10..12],
                        &path_str[12..14],
                        &path_str[14..19],
                    ]
                    .iter()
                    .collect::<PathBuf>();
                    let path = local_registry_path.join(v_path.as_path());
                    let r = tokio::fs::create_dir_all(path.clone().parent().unwrap())
                        .and_then(|_| async {
                            tokio::fs::write(
                                path,
                                PbChangelogEntry {
                                    key_mutations: ce
                                        .iter()
                                        .map(|km| {
                                            let mutation_type = if km.value.is_some() {
                                                MutationType::Set as i32
                                            } else {
                                                MutationType::Unset as i32
                                            };
                                            PbKeyMutation {
                                                key: km.key.clone(),
                                                value: km.value.clone().unwrap_or_default(),
                                                mutation_type,
                                            }
                                        })
                                        .collect(),
                                }
                                .encode_to_vec(),
                            )
                            .await
                        })
                        .await;
                    if let Err(e) = &r {
                        debug!("Storage err for {v}: {}", e);
                    } else {
                        debug!("Stored version {}", v);
                    }
                    r
                });
            });

            latest_version = latest_version.add(RegistryVersion::new(versions_count as u64));

            latest_certified_time = t.as_nanos_since_unix_epoch();
            debug!("Sync reached version {latest_version}");
        }
    }

    futures::future::join_all(updates).await;
    local_store.update_certified_time(latest_certified_time)?;
    Ok(())
}

async fn query_facts_db_guests(gitlab_client: AsyncGitlab, network: String) -> anyhow::Result<Vec<Guest>> {
    ::gitlab::api::raw(
        ::gitlab::api::projects::repository::files::FileRaw::builder()
            .ref_("refs/heads/main")
            .project("dfinity-lab/core/release")
            .file_path(format!("factsdb/data/{}_guests.csv", network))
            .build()
            .expect("failed to build API endpoint"),
    )
    .query_async(&gitlab_client)
    .await
    .map(|r| {
        csv::Reader::from_reader(r.as_slice())
            .deserialize()
            .map(|r| {
                let fdbg: FactsDBGuest = r.expect("record failed to parse");
                Guest::from(fdbg)
            })
            .collect::<Vec<_>>()
    })
    .or_else(|e| match e {
        ::gitlab::api::ApiError::Gitlab { msg } if msg.starts_with(&StatusCode::NOT_FOUND.as_u16().to_string()) => {
            warn!("No factsdb guests file found for network {network}: {msg}");
            Ok(vec![])
        }
        _ => Err(anyhow::anyhow!(e)),
    })
}

async fn poll(gitlab_client: AsyncGitlab, registry_state: Arc<RwLock<registry::RegistryState>>) {
    let mut print_counter = 0;
    let registry_canister = RegistryCanister::new(nns_nodes_urls());
    loop {
        sleep(Duration::from_secs(1)).await;
        let print_enabled = print_counter % 10 == 0;
        if print_enabled {
            info!("Updating registry");
        }
        let latest_version = if let Ok(v) = registry_canister.get_latest_version().await {
            v
        } else {
            continue;
        };
        if latest_version != registry_state.read().await.version() {
            let node_providers_result = query_ic_dashboard_list::<NodeProvidersResponse>("v3/node-providers").await;
            let network = network();
            let guests_result = query_facts_db_guests(gitlab_client.clone(), network.to_string()).await;
            let guests_result = if matches!(network, Network::Mainnet) {
                let guests_result_old =
                    query_facts_db_guests(gitlab_client.clone(), network.legacy_name().to_string()).await;
                guests_result.and_then(|guests_decentralized| {
                    guests_result_old.map(|guests_old| {
                        guests_decentralized
                            .into_iter()
                            .chain(guests_old.into_iter())
                            .collect::<Vec<_>>()
                    })
                })
            } else {
                guests_result
            };
            match (node_providers_result, guests_result) {
                (Ok(node_providers_response), Ok(guests_list)) => {
                    let mut registry_state = registry_state.write().await;
                    let update = registry_state
                        .update(node_providers_response.node_providers, guests_list)
                        .await;
                    if let Err(e) = update {
                        warn!("failed state update: {}", e);
                    }
                    if print_enabled {
                        info!("Updated registry state to version {}", registry_state.version());
                    }
                }
                (Err(e), _) => {
                    warn!("Failed querying IC dashboard {}", e);
                }
                (_, Err(e)) => {
                    warn!("Failed querying guests file: {}", e);
                }
            }
        } else if print_enabled {
            info!(
                "Skipping update. Registry already on latest version: {}",
                registry_state.read().await.version()
            )
        }
        print_counter += 1;
    }
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

fn network() -> Network {
    Network::from_str(&std::env::var("NETWORK").expect("Missing NETWORK environment variable"))
        .expect("Invalid network")
}

fn local_registry_path() -> PathBuf {
    match std::env::var("LOCAL_REGISTRY_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => match dirs::cache_dir() {
            Some(cache_dir) => cache_dir,
            None => PathBuf::from("/tmp"),
        },
    }
    .join("ic-registry-cache")
    .join(network().to_string())
    .join("local_registry")
}
