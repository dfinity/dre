use std::collections::{BTreeMap, HashSet};
use std::{cmp::Ordering, sync::Arc};

use candid::Principal;
use ic_interfaces_registry::{empty_zero_registry_record, RegistryClientVersionedResult, RegistryVersionedRecord, ZERO_REGISTRY_VERSION};
use ic_interfaces_registry::{RegistryDataProvider, RegistryTransportRecord};
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX};
use ic_registry_transport::{deserialize_get_changes_since_response, pb::v1::RegistryDelta, serialize_get_changes_since_request};
use ic_types::registry::{RegistryClientError, RegistryDataProviderError};
use ic_types::{RegistryVersion, Time};
use itertools::Itertools;
use lazy_static::lazy_static;
use prost::Message;

use crate::stable_memory::{self, MAX_STRING, MIN_STRING};
use crate::types::RegistryKey;

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

    fn new() -> Self {
        Self
    }
}

impl RegistryDataProvider for StableMemoryStore {
    fn get_updates_since(&self, version: RegistryVersion) -> Result<Vec<RegistryTransportRecord>, RegistryDataProviderError> {
        stable_memory::REGISTRY_STORED.with_borrow(|local_registry| {
            let next_version = version.get() + 1;
            let changelog = local_registry
                .range(RegistryKey{ version: next_version, key: MIN_STRING.clone()}..)
                .collect_vec();


            ic_cdk::println!("changelog {:?}", changelog);

            let res: Vec<_> = changelog
                .iter()
                .map(|(RegistryKey{ version, key}, value)| RegistryTransportRecord {
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
    registry_cache: FakeRegistryClient
}
impl LocalRegistry {
    pub fn new() -> Self {
        let local_store = Box::new(StableMemoryStore::new());
        let registry_cache = FakeRegistryClient::new(local_store);

        LocalRegistry { 
            registry_cache
        }
    }

    pub async fn sync_registry_stored(&self) -> anyhow::Result<()> {

        ic_cdk::println!("resync");
        let sync_ts = ic_cdk::api::time();
        let registry_client = RegistryCaller::new();
        let registry_stored = Arc::new(StableMemoryStore::new());
        let mut last_registry_version = 0;
        self.registry_cache.update_to_latest_version();
    
        loop {
            let remote_latest_version = registry_client.get_latest_version().await;
            let local_latest_version = self.get_latest_version().get();
    
            match local_latest_version.cmp(&remote_latest_version) {
                Ordering::Less => {
                    ic_cdk::println!("Registry version local {} < remote {}", local_latest_version, remote_latest_version);
                }
                Ordering::Equal => {
                    ic_cdk::println!("Local Registry version {} is up to date", local_latest_version);
                    break;
                }
                Ordering::Greater => {
                    let message = format!(
                        "Registry version local {} > remote {}, this should never happen",
                        local_latest_version, remote_latest_version
                    );
    
                    ic_cdk::trap(message.as_str());
                }
            }
    
            if let Ok(mut registry_records) = registry_client.get_changes_since(local_latest_version).await {
                registry_records.sort_by_key(|tr| tr.version);
    
                registry_records.into_iter().for_each(|record| {
                    if RETAINED_KEYS.iter().any(|&prefix| record.key.starts_with(prefix)) {
                        last_registry_version = record.version.get();
                        registry_stored.insert_registry_record(RegistryKey{ version: record.version.get(), key: record.key}, record.value);
                    }
                });
            }
            self.registry_cache.update_to_latest_version();
        }

        ic_cdk::println!("inserted all records");

        registry_stored.insert_registry_version(sync_ts, last_registry_version);
        Ok(())
    }

    fn get_family_entries_of_version<T: Message + Default>(
        &self,
        prefix: &str,
        version: RegistryVersion,
    ) -> anyhow::Result<BTreeMap<String, (u64, T)>> {
        let prefix_length = prefix.len();
        Ok(self
            .get_key_family(prefix, version)?
            .iter()
            .filter_map(|key| {
                let r = self
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

impl RegistryClient for LocalRegistry {
    fn get_versioned_value(&self, key: &str, version: RegistryVersion) -> ic_interfaces_registry::RegistryClientVersionedResult<Vec<u8>> {
        self.registry_cache.get_versioned_value(key, version)
    }

    fn get_key_family(&self, key_prefix: &str, version: RegistryVersion) -> Result<Vec<String>, RegistryClientError> {
        self.registry_cache.get_key_family(key_prefix, version)
    }

    fn get_latest_version(&self) -> RegistryVersion {
        self.registry_cache.get_latest_version()
    }

    fn get_version_timestamp(&self, registry_version: RegistryVersion) -> Option<ic_types::Time> {
        self.registry_cache.get_version_timestamp(registry_version)
    }
}

struct RegistryCaller;
impl RegistryCaller {
    pub fn new() -> Self {
        Self
    }

    async fn get_latest_version(&self) -> u64 {
        // ic_nns_common::registry::get_latest_version().await

        5000
    }
    
    async fn get_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryTransportRecord>> {
        // let buff = serialize_get_changes_since_request(version).unwrap();
        // let response = ic_cdk::api::call::call_raw(Principal::from(REGISTRY_CANISTER_ID), "get_changes_since", buff, 0)
        //     .await
        //     .unwrap();
        // let (registry_delta, _) = deserialize_get_changes_since_response(response).unwrap();
        // let registry_transport_record = registry_deltas_to_registry_transport_records(registry_delta)?;

        let registry_transport_record: Vec<RegistryTransportRecord> = (0..=5000).filter_map(|i| {
            if i > version {
                Some(RegistryVersionedRecord {
                    key: format!("node_record_{}", i),
                    version: RegistryVersion::from(i),
                    value: Some(vec![i as u8; 5]), 
                })
            } else {
                None
            }
        }).collect();

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
