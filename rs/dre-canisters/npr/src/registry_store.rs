mod tests;

use candid::Principal;
use ic_interfaces_registry::{
    empty_zero_registry_record, RegistryClient, RegistryClientResult, RegistryClientVersionedResult, RegistryTransportRecord, ZERO_REGISTRY_VERSION,
};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{StableBTreeMap, Storable};
use ic_types::registry::RegistryClientError;
use ic_types::RegistryVersion;
use itertools::Itertools;
use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::rc::Rc;

// This value is set as 2 times the max key size present in the registry
const MAX_REGISTRY_KEY_SIZE: u32 = 200;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug)]
pub struct StorableRegistryKey {
    pub key: String,
    pub version: RegistryVersion,
}
impl StorableRegistryKey {
    pub fn new(key: String, version: RegistryVersion) -> Self {
        Self { key, version }
    }
}

impl Storable for StorableRegistryKey {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut storable_key = vec![];
        let version_b = self.version.get().to_be_bytes().to_vec();
        let key_b = self.key.as_bytes().to_vec();

        storable_key.extend_from_slice(&version_b);
        storable_key.extend_from_slice(&key_b);

        Cow::Owned(storable_key)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let (version_bytes, key_bytes) = bytes.split_at(8);

        let version_u64 = u64::from_be_bytes(version_bytes.try_into().expect("Invalid version bytes"));
        let version = RegistryVersion::new(version_u64);
        let key = String::from_utf8(key_bytes.to_vec()).expect("Invalid UTF-8 in key");
        Self { key, version }
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_REGISTRY_KEY_SIZE + size_of::<u64>() as u32,
        is_fixed_size: false,
    };
}

#[derive(Clone, Debug)]
pub struct StorableRegistryValue(pub Option<Vec<u8>>);

impl Storable for StorableRegistryValue {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(Option::from_bytes(bytes))
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub struct CanisterRegistryStore<Memory>
where
    Memory: ic_stable_structures::Memory,
{
    local_registry: StableBTreeMap<StorableRegistryKey, StorableRegistryValue, Memory>,
}

impl<Memory> CanisterRegistryStore<Memory>
where
    Memory: ic_stable_structures::Memory,
{
    pub fn init(memory: Memory) -> Self {
        Self {
            local_registry: StableBTreeMap::init(memory),
        }
    }

    fn add_deltas(&mut self, deltas: Vec<RegistryDelta>) -> anyhow::Result<()> {
        for delta in deltas {
            let string_key = std::str::from_utf8(&delta.key[..]).map_err(|_| anyhow::anyhow!("Failed to convert key {:?} to string", delta.key))?;

            for value in delta.values.into_iter() {
                self.local_registry.insert(
                    StorableRegistryKey {
                        key: string_key.to_string(),
                        version: RegistryVersion::from(value.version),
                    },
                    StorableRegistryValue(if value.deletion_marker { None } else { Some(value.value) }),
                );
            }
        }
        Ok(())
    }

    pub fn get_versioned_value(&self, key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>> {
        if version == ZERO_REGISTRY_VERSION {
            return Ok(empty_zero_registry_record(key));
        }
        if self.get_latest_version() < version {
            return Err(RegistryClientError::VersionNotAvailable { version });
        }

        let search_key = StorableRegistryKey::new(key.to_string(), version);

        let result = self
            .local_registry
            .range(..=search_key)
            .rev()
            .find(|(stored_key, _)| stored_key.key == key)
            .map(|(_, value)| RegistryTransportRecord {
                key: key.to_string(),
                version,
                value: value.0,
            })
            .unwrap_or_else(|| empty_zero_registry_record(key));
        Ok(result)
    }

    pub fn get_key_family(&self, key_prefix: &str, version: RegistryVersion) -> Result<Vec<String>, RegistryClientError> {
        if version == ZERO_REGISTRY_VERSION {
            return Ok(vec![]);
        }
        if self.get_latest_version() < version {
            return Err(RegistryClientError::VersionNotAvailable { version });
        }

        let first_matching_key = StorableRegistryKey {
            key: key_prefix.to_string(),
            ..Default::default()
        };

        let records_history = self
            .local_registry
            .range(first_matching_key..)
            .filter(|(storable_key, _)| storable_key.version <= version)
            .take_while(|(storable_key, _)| storable_key.key.starts_with(key_prefix));

        let mut effective_records = BTreeMap::new();

        for (stored_key, value) in records_history {
            effective_records.insert(stored_key.key, value.0);
        }

        let results = effective_records
            .into_iter()
            .filter_map(|(key, value)| value.is_some().then_some(key))
            .collect();
        Ok(results)
    }

    pub fn get_latest_version(&self) -> RegistryVersion {
        self.local_registry.keys().map(|k| k.version).max().unwrap_or(ZERO_REGISTRY_VERSION)
    }

    async fn get_registry_changes_since(version: u64) -> anyhow::Result<Vec<RegistryDelta>> {
        let buff = serialize_get_changes_since_request(version)?;
        let response = ic_cdk::api::call::call_raw(Principal::from(REGISTRY_CANISTER_ID), "get_changes_since", buff, 0)
            .await
            .map_err(|(code, msg)| (code as i32, msg))
            .unwrap();
        let (registry_delta, _) = deserialize_get_changes_since_response(response)?;
        Ok(registry_delta)
    }

    pub async fn sync_registry_stored(rc_self: Rc<RefCell<Self>>) -> anyhow::Result<()> {
        let mut update_registry_version = rc_self.borrow().get_latest_version().get();

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

            let remote_deltas = Self::get_registry_changes_since(update_registry_version).await?;

            update_registry_version = remote_deltas
                .iter()
                .flat_map(|delta| delta.values.iter().map(|v| v.version))
                .max()
                .unwrap_or(update_registry_version);
            rc_self.borrow_mut().add_deltas(remote_deltas)?;
        }
        Ok(())
    }
}
