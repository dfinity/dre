use candid::Principal;
use ic_interfaces_registry::{RegistryDataProvider, RegistryTransportRecord, ZERO_REGISTRY_VERSION};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX};
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};
use ic_stable_structures::memory_manager::VirtualMemory;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use ic_types::registry::RegistryDataProviderError;
use ic_types::RegistryVersion;
use itertools::Itertools;
use lazy_static::lazy_static;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::marker::PhantomData;

lazy_static! {
    static ref RETAINED_KEYS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert(NODE_RECORD_KEY_PREFIX);
        set.insert(NODE_OPERATOR_RECORD_KEY_PREFIX);
        set.insert(DATA_CENTER_KEY_PREFIX);
        set
    };
}

pub struct StorableRegistryValue(Option<Vec<u8>>);

impl Storable for StorableRegistryValue {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(Option::from_bytes(bytes))
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct StorableRegistryKey {
    pub version: u64,
    pub key: String,
}

impl Storable for StorableRegistryKey {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut storable_key = vec![];
        let version_b = self.version.to_be_bytes().to_vec();
        let key_b = self.key.as_bytes().to_vec();

        storable_key.extend_from_slice(&version_b);
        storable_key.extend_from_slice(&key_b);

        Cow::Owned(storable_key)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let (version_bytes, key_bytes) = bytes.split_at(8);
        let version = u64::from_be_bytes(version_bytes.try_into().expect("Invalid version bytes"));
        let key = String::from_utf8(key_bytes.to_vec()).expect("Invalid UTF-8 in key");

        Self { version, key }
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: 203 + 64,
        is_fixed_size: false,
    };
}
pub trait StableMemoryBorrower: Send + Sync {
    fn with_borrow<R>(f: impl FnOnce(&StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VirtualMemory<DefaultMemoryImpl>>) -> R) -> R;
    fn with_borrow_mut<R>(
        f: impl FnOnce(&mut StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VirtualMemory<DefaultMemoryImpl>>) -> R,
    ) -> R;
}
pub struct CanisterDataProvider<S: StableMemoryBorrower> {
    keys_to_retain: Option<HashSet<String>>,
    _store: PhantomData<S>,
}

impl<S: StableMemoryBorrower> CanisterDataProvider<S> {
    pub fn new(keys_to_retain: Option<HashSet<String>>) -> Self {
        Self {
            keys_to_retain,
            _store: PhantomData,
        }
    }

    async fn get_registry_changes_since(&self, version: u64) -> anyhow::Result<Vec<RegistryDelta>> {
        let buff = serialize_get_changes_since_request(version)?;
        let response = ic_cdk::api::call::call_raw(Principal::from(REGISTRY_CANISTER_ID), "get_changes_since", buff, 0)
            .await
            .map_err(|(code, msg)| (code as i32, msg))
            .unwrap();
        let (registry_delta, _) = deserialize_get_changes_since_response(response)?;
        Ok(registry_delta)
    }

    fn get_latest_version(&self) -> Option<u64> {
        S::with_borrow(|local_registry| local_registry.last_key_value().map(|(k, _)| k.version))
    }

    pub async fn sync_registry_stored(&self) -> anyhow::Result<()> {
        let mut update_registry_version = self.get_latest_version().unwrap_or(ZERO_REGISTRY_VERSION.get());

        loop {
            let remote_latest_version = ic_nns_common::registry::get_latest_version().await;

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

            if let Ok(deltas) = self.get_registry_changes_since(update_registry_version).await {
                update_registry_version = deltas
                    .iter()
                    .flat_map(|delta| delta.values.iter().map(|v| v.version))
                    .max()
                    .unwrap_or(update_registry_version);

                for delta in deltas {
                    let string_key =
                        std::str::from_utf8(&delta.key[..]).map_err(|_| anyhow::anyhow!("Failed to convert key {:?} to string", delta.key))?;

                    if RETAINED_KEYS.iter().any(|&prefix| string_key.starts_with(prefix)) {
                        for value in delta.values.into_iter() {
                            S::with_borrow_mut(|local_registry| {
                                local_registry.insert(
                                    StorableRegistryKey {
                                        key: string_key.to_string(),
                                        version: value.version,
                                    },
                                    StorableRegistryValue(value.deletion_marker.then(|| value.value)),
                                );
                            })
                        }
                    }
                }
            };
        }
        Ok(())
    }
}

impl<S: StableMemoryBorrower> RegistryDataProvider for CanisterDataProvider<S> {
    fn get_updates_since(&self, version: RegistryVersion) -> Result<Vec<RegistryTransportRecord>, RegistryDataProviderError> {
        S::with_borrow(|local_registry| {
            let start_key = StorableRegistryKey {
                version: version.get(),
                ..Default::default()
            };

            let from_start_key = local_registry
                .range(start_key..)
                .map(|(storable_key, value)| RegistryTransportRecord {
                    version: RegistryVersion::from(storable_key.version),
                    key: storable_key.key,
                    value: value.0,
                })
                .collect_vec();

            Ok(from_start_key)
        })
    }
}
