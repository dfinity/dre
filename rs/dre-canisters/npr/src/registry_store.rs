use crate::registry_store_types::{StorableRegistryKey, StorableRegistryValue};
use crate::storage::VM;
use candid::Principal;
use ic_interfaces_registry::{empty_zero_registry_record, RegistryClientVersionedResult, RegistryTransportRecord, ZERO_REGISTRY_VERSION};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};
use ic_stable_structures::StableBTreeMap;
use ic_types::registry::RegistryClientError;
use ic_types::RegistryVersion;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub type StableLocalRegistry = StableBTreeMap<StorableRegistryKey, StorableRegistryValue, VM>;

pub trait RegistryData {
    fn with_local_registry<R>(f: impl FnOnce(&StableLocalRegistry) -> R) -> R;
    fn with_local_registry_mut<R>(f: impl FnOnce(&mut StableLocalRegistry) -> R) -> R;
}

pub struct CanisterRegistryStore<D: RegistryData> {
    _registry_data: PhantomData<D>,
}

impl<D: RegistryData> CanisterRegistryStore<D> {
    fn add_deltas(deltas: Vec<RegistryDelta>) -> anyhow::Result<()> {
        for delta in deltas {
            let string_key = std::str::from_utf8(&delta.key[..]).map_err(|_| anyhow::anyhow!("Failed to convert key {:?} to string", delta.key))?;

            D::with_local_registry_mut(|local_registry| {
                for value in delta.values {
                    local_registry.insert(
                        StorableRegistryKey {
                            key: string_key.to_string(),
                            version: RegistryVersion::from(value.version),
                        },
                        StorableRegistryValue(if value.deletion_marker { None } else { Some(value.value) }),
                    );
                }
            });
        }
        Ok(())
    }

    pub fn get_versioned_value(key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>> {
        if version == ZERO_REGISTRY_VERSION {
            return Ok(empty_zero_registry_record(key));
        }
        if Self::get_latest_version() < version {
            return Err(RegistryClientError::VersionNotAvailable { version });
        }

        let search_key = StorableRegistryKey::new(key.to_string(), version);

        let result = D::with_local_registry_mut(|local_registry| {
            local_registry
                .range(..=search_key)
                .rev()
                .find(|(stored_key, _)| stored_key.key == key)
                .map(|(_, value)| RegistryTransportRecord {
                    key: key.to_string(),
                    version,
                    value: value.0,
                })
                .unwrap_or_else(|| empty_zero_registry_record(key))
        });
        Ok(result)
    }

    pub fn get_key_family(key_prefix: &str, version: RegistryVersion) -> Result<Vec<String>, RegistryClientError> {
        if version == ZERO_REGISTRY_VERSION {
            return Ok(vec![]);
        }
        if Self::get_latest_version() < version {
            return Err(RegistryClientError::VersionNotAvailable { version });
        }

        let first_matching_key = StorableRegistryKey {
            key: key_prefix.to_string(),
            ..Default::default()
        };

        let effective_records = D::with_local_registry(|local_registry| {
            let mut effective_records = BTreeMap::new();

            let records_history = local_registry
                .range(first_matching_key..)
                .filter(|(storable_key, _)| storable_key.version <= version)
                .take_while(|(storable_key, _)| storable_key.key.starts_with(key_prefix));

            for (stored_key, value) in records_history {
                effective_records.insert(stored_key.key, value.0);
            }

            effective_records
        });

        let results = effective_records
            .into_iter()
            .filter_map(|(key, value)| value.is_some().then_some(key))
            .collect();
        Ok(results)
    }

    pub fn get_latest_version() -> RegistryVersion {
        D::with_local_registry(|local_registry| local_registry.keys().map(|k| k.version).max().unwrap_or(ZERO_REGISTRY_VERSION))
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

    pub async fn sync_registry_stored() -> anyhow::Result<()> {
        let mut update_registry_version = Self::get_latest_version().get();

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

            Self::add_deltas(remote_deltas)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
