use futures::TryFutureExt;
use ic_registry_client_helpers::{node::SubnetId, node_operator::PrincipalId};
use std::{
    error::Error,
    ops::Add,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crossbeam_channel::Receiver;
use ic_interfaces_registry::{RegistryClient, RegistryValue, ZERO_REGISTRY_VERSION};
use ic_protobuf::registry::crypto::v1::PublicKey;
use ic_registry_client::client::{RegistryVersion, ThresholdSigPublicKey};
use ic_registry_client_fake::FakeRegistryClient;
use ic_registry_common_proto::pb::local_store::v1::{ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType};
use ic_registry_keys::{make_crypto_threshold_signing_pubkey_key, ROOT_SUBNET_ID_KEY};
use ic_registry_local_store::{Changelog, ChangelogEntry, KeyMutation, LocalStoreImpl};
use ic_registry_nns_data_provider::registry::RegistryCanister;
use slog::{debug, error, info, warn, Logger};
use std::fmt::{Display, Formatter};
use url::Url;

#[derive(Debug)]
pub enum SyncError {
    Interrupted,
    PublicKey(String),
}

impl Error for SyncError {}

impl Display for SyncError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

pub async fn sync_local_registry(
    log: Logger,
    local_path: PathBuf,
    nns_urls: &[Url],
    use_current_version: bool,
    public_key: Option<ThresholdSigPublicKey>,
    stop_signal: &Receiver<()>,
) -> Result<(), SyncError> {
    let start = Instant::now();
    let local_store = Arc::new(LocalStoreImpl::new(local_path.clone()));
    let registry_canister = RegistryCanister::new(nns_urls.to_vec());

    if stop_signal.try_recv().is_ok() {
        // Interrupted early.  Let's get out of here.
        return Err(SyncError::Interrupted);
    }

    let mut latest_version = if !Path::new(&local_path).exists() {
        ZERO_REGISTRY_VERSION
    } else {
        let registry_cache = FakeRegistryClient::new(local_store.clone());
        registry_cache.update_to_latest_version();
        registry_cache.get_latest_version()
    };
    debug!(log, "Syncing registry version from version : {}", latest_version);

    if use_current_version && latest_version != ZERO_REGISTRY_VERSION {
        debug!(log, "Skipping syncing with registry, using local version");
        return Ok(());
    } else if use_current_version {
        warn!(log, "Unable to use current version of registry since its a zero registry version");
    }

    let mut updates = vec![];
    let nns_public_key = match public_key {
        Some(pk) => pk,
        _ => match get_nns_public_key(&registry_canister).await {
            Ok(key) => key,
            Err(e) => {
                let network_name = local_path.file_name().unwrap().to_str().unwrap();
                debug!(log, "Unable to fetch public key for network {}: {:?}", network_name, e);
                return Err(SyncError::PublicKey(e.to_string()));
            }
        },
    };

    loop {
        if stop_signal.try_recv().is_ok() {
            // Interrupted.  Let's get out of here.
            return Err(SyncError::Interrupted);
        }

        if match registry_canister.get_latest_version().await {
            Ok(v) => {
                debug!(log, "Latest registry version: {}", v);
                v == latest_version.get()
            }
            Err(e) => {
                error!(log, "Failed to get latest registry version: {}", e);
                false
            }
        } {
            break;
        }

        if let Ok((mut initial_records, _, _)) = registry_canister.get_certified_changes_since(latest_version.get(), &nns_public_key).await {
            initial_records.sort_by_key(|r| r.version);
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
                let local_registry_path = local_path.clone();
                updates.push(async move {
                    let path_str = format!("{:016x}.pb", v.get());
                    let v_path = &[&path_str[0..10], &path_str[10..12], &path_str[12..14], &path_str[14..19]]
                        .iter()
                        .collect::<PathBuf>();

                    let path = local_registry_path.join(v_path.as_path());
                    tokio::fs::create_dir_all(path.clone().parent().unwrap())
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
                        .await
                })
            });

            latest_version = latest_version.add(RegistryVersion::new(versions_count as u64));

            debug!(log, "Initial sync reached version {}", latest_version);
        }
    }

    futures::future::join_all(updates).await;
    info!(log, "Synced all registry versions in : {:?}", start.elapsed());
    Ok(())
}

async fn get_nns_public_key(registry_canister: &RegistryCanister) -> anyhow::Result<ThresholdSigPublicKey> {
    let (nns_subnet_id_vec, _) = registry_canister
        .get_value(ROOT_SUBNET_ID_KEY.as_bytes().to_vec(), None)
        .await
        .map_err(|e| anyhow::format_err!("failed to get root subnet: {}", e))?;
    let nns_subnet_id = ic_protobuf::types::v1::SubnetId::decode(nns_subnet_id_vec.as_slice())?;
    let (nns_pub_key_vec, _) = registry_canister
        .get_value(
            make_crypto_threshold_signing_pubkey_key(SubnetId::new(PrincipalId::try_from(nns_subnet_id.principal_id.unwrap().raw).unwrap()))
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

pub async fn nns_reachable(nns_urls: Vec<Url>) -> bool {
    if nns_urls.is_empty() {
        return false;
    }

    let registry_canister = RegistryCanister::new(nns_urls);

    get_nns_public_key(&registry_canister).await.is_ok()
}
