use crate::stable_memory;
use candid::Principal;
use ic_interfaces_registry::{RegistryDataProvider, RegistryTransportRecord, ZERO_REGISTRY_VERSION};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_common_proto::pb::proto_registry::v1::ProtoRegistryRecord;
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX};
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};
use ic_stable_structures::memory_manager::VirtualMemory;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableVec, Storable};
use ic_types::registry::RegistryDataProviderError;
use ic_types::RegistryVersion;
use itertools::Itertools;
use lazy_static::lazy_static;
use prost::Message;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;

lazy_static! {
    static ref RETAINED_KEYS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert(NODE_RECORD_KEY_PREFIX);
        set.insert(NODE_OPERATOR_RECORD_KEY_PREFIX);
        set.insert(DATA_CENTER_KEY_PREFIX);
        set
    };
}

pub struct StableRegistryRecord(ProtoRegistryRecord);

impl Storable for StableRegistryRecord {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::from(self.0.encode_to_vec())
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(ProtoRegistryRecord::decode(&bytes[..]).unwrap())
    }

    const BOUND: Bound = Bound::Unbounded;
}
type Memory = VirtualMemory<DefaultMemoryImpl>;

#[derive(Default)]
pub struct StableMemoryStore;
impl StableMemoryBorrower for StableMemoryStore {
    fn with_borrow<R>(f: impl FnOnce(&StableVec<StableRegistryRecord, Memory>) -> R) -> R {
        stable_memory::REGISTRY.with_borrow(|registry_stored| f(registry_stored))
    }

    fn with_borrow_mut<R>(f: impl FnOnce(&mut StableVec<StableRegistryRecord, Memory>) -> R) -> R {
        stable_memory::REGISTRY.with_borrow_mut(|registry_stored| f(registry_stored))
    }
}

trait StableMemoryBorrower: Send + Sync {
    fn with_borrow<R>(f: impl FnOnce(&StableVec<StableRegistryRecord, Memory>) -> R) -> R;
    fn with_borrow_mut<R>(f: impl FnOnce(&mut StableVec<StableRegistryRecord, Memory>) -> R) -> R;
}
pub struct CanisterDataProvider<S: StableMemoryBorrower> {
    registry_caller: RegistryCaller,
    keys_to_retain: Option<HashSet<String>>,
    stable_memory_store: S,
}

impl<S: StableMemoryBorrower> CanisterDataProvider<S> {
    pub fn new(keys_to_retain: Option<HashSet<String>>, stable_memory_store: S) -> Self {
        Self {
            keys_to_retain,
            registry_caller: RegistryCaller::default(),
            stable_memory_store,
        }
    }

    fn get_latest_version(&self) -> Option<u64> {
        S::with_borrow(|local_registry| local_registry.iter().last().map(|last_record| last_record.0.version))
    }

    pub async fn sync_registry_stored(&self) -> anyhow::Result<()> {
        let mut update_registry_version = self.get_latest_version().unwrap_or(ZERO_REGISTRY_VERSION.get());
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

                update_registry_version = registry_records.last().map(|record| record.version.get()).unwrap();

                registry_records.into_iter().for_each(|record| {
                    if RETAINED_KEYS.iter().any(|&prefix| record.key.starts_with(prefix)) {
                        self.insert_registry_record(ProtoRegistryRecord {
                            key: record.key,
                            version: record.version.get(),
                            value: record.value,
                        });
                    }
                });
            }
        }
        Ok(())
    }

    fn insert_registry_record(&self, record: ProtoRegistryRecord) {
        S::with_borrow_mut(|local_registry| {
            local_registry.push(&StableRegistryRecord(record)).unwrap();
        })
    }
}

impl<S: StableMemoryBorrower> RegistryDataProvider for CanisterDataProvider<S> {
    fn get_updates_since(&self, version: RegistryVersion) -> Result<Vec<RegistryTransportRecord>, RegistryDataProviderError> {
        S::with_borrow(|local_registry| {
            let since_version = version.get();
            let updates = local_registry
                .iter()
                .filter(|record| record.0.version > since_version)
                .map(|record| RegistryTransportRecord {
                    version: RegistryVersion::from(record.0.version),
                    key: record.0.key,
                    value: record.0.value,
                })
                .collect_vec();

            Ok(updates)
        })
    }
}

#[derive(Default)]
struct RegistryCaller;
impl RegistryCaller {
    async fn get_latest_version(&self) -> u64 {
        ic_nns_common::registry::get_latest_version().await
    }

    async fn get_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryTransportRecord>> {
        let buff = serialize_get_changes_since_request(version)?;
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
