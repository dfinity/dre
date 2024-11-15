use std::collections::{BTreeMap, HashSet};
use std::{cmp::Ordering, sync::Arc};

use candid::Principal;
use ic_interfaces_registry::RegistryClient;
use ic_interfaces_registry::{RegistryDataProvider, RegistryTransportRecord};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_canister_client::CanisterRegistryClient;
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX};
use ic_registry_transport::{deserialize_get_changes_since_response, pb::v1::RegistryDelta, serialize_get_changes_since_request};
use ic_types::registry::RegistryDataProviderError;
use ic_types::RegistryVersion;
use itertools::Itertools;
use lazy_static::lazy_static;
use prost::Message;
use trustworthy_node_metrics_types::types::RegistryKey;

use crate::chrono_utils::DateTimeRange;
use crate::stable_memory::{self, MIN_STRING};

lazy_static! {
    static ref RETAINED_KEYS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert(NODE_RECORD_KEY_PREFIX);
        set.insert(NODE_OPERATOR_RECORD_KEY_PREFIX);
        set.insert(DATA_CENTER_KEY_PREFIX);
        set
    };
}

struct StableMemoryStore;
impl StableMemoryStore {
    fn insert_registry_record(&self, key: RegistryKey, value: Option<Vec<u8>>) {
        stable_memory::REGISTRY_STORED.with_borrow_mut(|registry_stored| registry_stored.insert(key, value));
    }

    fn insert_registry_version(&self, ts: u64, version: u64) {
        stable_memory::TS_REGISTRY_VERSIONS.with_borrow_mut(|versions| versions.insert(ts, version));
    }

    fn get_registry_versions(&self, start_ts: u64, end_ts: u64) -> Vec<u64> {
        stable_memory::TS_REGISTRY_VERSIONS.with_borrow(|versions| versions.range(start_ts..=end_ts).map(|(_, version)| version).collect_vec())
    }

    fn new() -> Self {
        Self
    }
}

impl RegistryDataProvider for StableMemoryStore {
    fn get_updates_since(&self, version: RegistryVersion) -> Result<Vec<RegistryTransportRecord>, RegistryDataProviderError> {
        stable_memory::REGISTRY_STORED.with_borrow(|local_registry| {
            let next_version = version.get() + 1;
            let changelog = local_registry
                .range(
                    RegistryKey {
                        version: next_version,
                        key: MIN_STRING.clone(),
                    }..,
                )
                .collect_vec();

            let res: Vec<_> = changelog
                .iter()
                .map(|(RegistryKey { version, key }, value)| RegistryTransportRecord {
                    version: RegistryVersion::from(*version),
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect();
            Ok(res)
        })
    }
}

pub struct LocalRegistry {
    registry_cache: CanisterRegistryClient,
    registry_caller: RegistryCaller,
    local_store: Arc<StableMemoryStore>,
}

impl Default for LocalRegistry {
    fn default() -> Self {
        let local_store = Arc::new(StableMemoryStore::new());
        LocalRegistry {
            registry_cache: CanisterRegistryClient::new(local_store.clone()),
            registry_caller: RegistryCaller::new(),
            local_store,
        }
    }
}

impl LocalRegistry {
    pub async fn sync_registry_stored(&self) -> anyhow::Result<()> {
        let sync_ts: u64 = ic_cdk::api::time();
        self.registry_cache.update_to_latest_version();
        let mut update_registry_version = self.registry_cache.get_latest_version().get();

        loop {
            let remote_latest_version = self.registry_caller.get_latest_version().await;

            ic_cdk::println!("local version: {} remote version: {}", update_registry_version, remote_latest_version);

            match update_registry_version.cmp(&remote_latest_version) {
                Ordering::Less => {
                    ic_cdk::println!("Registry version local {} < remote {}", update_registry_version, remote_latest_version);
                }
                Ordering::Equal => {
                    ic_cdk::println!("Local Registry version {} is up to date", update_registry_version);
                    break;
                }
                Ordering::Greater => {
                    let message = format!(
                        "Registry version local {} > remote {}, this should never happen",
                        update_registry_version, remote_latest_version
                    );

                    ic_cdk::trap(message.as_str());
                }
            }

            if let Ok(mut registry_records) = self.registry_caller.get_changes_since(update_registry_version).await {
                registry_records.sort_by_key(|tr| tr.version);

                update_registry_version = registry_records.last().map(|redord| redord.version.get()).unwrap();

                registry_records.into_iter().for_each(|record| {
                    if RETAINED_KEYS.iter().any(|&prefix| record.key.starts_with(prefix)) {
                        let version = record.version.get();

                        self.local_store.insert_registry_version(sync_ts, version);
                        self.local_store
                            .insert_registry_record(RegistryKey { version, key: record.key }, record.value);
                    }
                });
            }
            self.registry_cache.update_to_latest_version();
        }

        Ok(())
    }

    pub fn registry_versions_in_range(&self, range: &DateTimeRange) -> Vec<u64> {
        let start_ts = range.start_timestamp_nanos();
        let end_ts = range.end_timestamp_nanos();

        self.local_store.get_registry_versions(start_ts, end_ts)
    }

    pub fn get_latest_version(&self) -> RegistryVersion {
        self.registry_cache.get_latest_version()
    }

    pub fn get_versioned_value<T: Message + Default>(&self, key: &str, version: RegistryVersion) -> anyhow::Result<T> {
        let r = self.registry_cache.get_versioned_value(key, version)?;

        Ok(r.as_ref().map(|v| T::decode(v.as_slice()).expect("Invalid registry value")).unwrap())
    }

    pub fn get_family_entries_of_version<T: Message + Default>(
        &self,
        prefix: &str,
        version: RegistryVersion,
    ) -> anyhow::Result<BTreeMap<String, (u64, T)>> {
        ic_cdk::println!("lastest: {}", self.registry_cache.get_latest_version());
        let prefix_length = prefix.len();
        Ok(self
            .registry_cache
            .get_key_family(prefix, version)?
            .iter()
            .filter_map(|key| {
                let r = self
                    .registry_cache
                    .get_versioned_value(key, version)
                    .unwrap_or_else(|_| panic!("Failed to get entry {} for type {}", key, std::any::type_name::<T>()));
                r.as_ref().map(|v| {
                    (
                        key[prefix_length..].to_string(),
                        (r.version.get(), T::decode(v.as_slice()).expect("Invalid registry value")),
                    )
                })
            })
            .collect())
    }
}

struct RegistryCaller;
impl RegistryCaller {
    pub fn new() -> Self {
        Self
    }

    async fn get_latest_version(&self) -> u64 {
        ic_nns_common::registry::get_latest_version().await
    }

    async fn get_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryTransportRecord>> {
        let buff = serialize_get_changes_since_request(version).unwrap();
        let response = ic_cdk::api::call::call_raw(Principal::from(REGISTRY_CANISTER_ID), "get_changes_since", buff, 0)
            .await
            .unwrap();
        let (registry_delta, _) = deserialize_get_changes_since_response(response).unwrap();
        let registry_transport_record = registry_deltas_to_registry_transport_records(registry_delta)?;
        Ok(registry_transport_record)
    }
}

fn registry_deltas_to_registry_transport_records(deltas: Vec<RegistryDelta>) -> anyhow::Result<Vec<RegistryTransportRecord>> {
    let mut records = Vec::new();
    for delta in deltas.into_iter() {
        let string_key = std::str::from_utf8(&delta.key[..])
            .map_err(|_| ic_registry_transport::Error::UnknownError(format!("Failed to convert key {:?} to string", delta.key)))?
            .to_string();

        for value in delta.values.into_iter() {
            records.push(RegistryTransportRecord {
                key: string_key.clone(),
                value: if value.deletion_marker { None } else { Some(value.value) },
                version: RegistryVersion::new(value.version),
            });
        }
    }
    records.sort_by(|lhs, rhs| lhs.version.cmp(&rhs.version).then_with(|| lhs.key.cmp(&rhs.key)));
    Ok(records)
}
