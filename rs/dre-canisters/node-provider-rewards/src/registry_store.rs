use crate::registry_store_types::{StorableRegistryKey, StorableRegistryValue};
use anyhow::anyhow;
use async_trait::async_trait;
use ic_interfaces_registry::{empty_zero_registry_record, RegistryClientVersionedResult, RegistryTransportRecord, ZERO_REGISTRY_VERSION};
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_stable_structures::StableBTreeMap;
use ic_types::registry::RegistryClientError;
use ic_types::RegistryVersion;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub type StableLocalRegistry<Memory> = StableBTreeMap<StorableRegistryKey, StorableRegistryValue, Memory>;

pub trait RegistryStoreData<Memory: ic_stable_structures::Memory> {
    fn with_local_registry<R>(f: impl FnOnce(&StableLocalRegistry<Memory>) -> R) -> R;
    fn with_local_registry_mut<R>(f: impl FnOnce(&mut StableLocalRegistry<Memory>) -> R) -> R;
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryCanisterClientError {
    #[error("RegistryTransportError: {0}")]
    RegistryTransportError(#[from] ic_registry_transport::Error),

    #[error("Call failed with code {0}: {1}")]
    CallError(u32, String),
}

#[async_trait]
pub trait RegistryCanisterClient {
    async fn registry_changes_since(&self, version: u64) -> Result<Vec<RegistryDelta>, RegistryCanisterClientError>;
}

pub struct CanisterRegistryStore<D: RegistryStoreData<Memory>, Memory>
where
    Memory: ic_stable_structures::Memory,
{
    _registry_data: PhantomData<D>,
    _memory: PhantomData<Memory>,
}

impl<D: RegistryStoreData<Memory>, Memory> CanisterRegistryStore<D, Memory>
where
    Memory: ic_stable_structures::Memory,
{
    fn add_deltas(deltas: Vec<RegistryDelta>) -> anyhow::Result<()> {
        for delta in deltas {
            let string_key = std::str::from_utf8(&delta.key[..])?;

            D::with_local_registry_mut(|local_registry| {
                for v in delta.values {
                    let registry_version = RegistryVersion::from(v.version);
                    let key = StorableRegistryKey::new(string_key.to_string(), registry_version);
                    let value = StorableRegistryValue(if v.deletion_marker { None } else { Some(v.value) });

                    local_registry.insert(key, value);
                }
            });
        }
        Ok(())
    }

    pub fn get_versioned_value(key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>> {
        if Self::local_latest_version() < version {
            return Err(RegistryClientError::VersionNotAvailable { version });
        }

        let search_key = StorableRegistryKey::new(key.to_string(), version);

        let result = D::with_local_registry(|local_registry| {
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

    fn key_family(
        key_prefix: &str,
        version_start: Option<RegistryVersion>,
        version_end: RegistryVersion,
    ) -> Result<Vec<String>, RegistryClientError> {
        if Self::local_latest_version() < version_end {
            return Err(RegistryClientError::VersionNotAvailable { version: version_end });
        }

        let first_matching_key = StorableRegistryKey {
            key: key_prefix.to_string(),
            ..Default::default()
        };

        let effective_records = D::with_local_registry(|local_registry| {
            let mut effective_records = BTreeMap::new();

            let records_history = local_registry
                .range(first_matching_key..)
                .filter(|(storable_key, _)| storable_key.version <= version_end)
                .take_while(|(storable_key, _)| storable_key.key.starts_with(key_prefix));

            for (stored_key, value) in records_history {
                if version_start.is_some_and(|version| stored_key.version > version) {
                    if value.0.is_some() {
                        effective_records.insert(stored_key.key, value.0);
                    }
                } else {
                    effective_records.insert(stored_key.key, value.0);
                }
            }

            effective_records
        });

        let results = effective_records
            .into_iter()
            .filter_map(|(key, value)| value.is_some().then_some(key))
            .collect();
        Ok(results)
    }

    pub fn get_key_family(key_prefix: &str, version: RegistryVersion) -> Result<Vec<String>, String> {
        if version == ZERO_REGISTRY_VERSION {
            return Ok(vec![]);
        }
        Self::key_family(key_prefix, None, version).map_err(|e| e.to_string())
    }

    pub fn get_key_family_between_versions(
        key_prefix: &str,
        version_start: RegistryVersion,
        version_end: RegistryVersion,
    ) -> Result<Vec<String>, String> {
        if version_start >= version_end {
            return Err(format!("Invalid version range: {} >= {}", version_start, version_end));
        }

        Self::key_family(key_prefix, Some(version_start), version_end).map_err(|e| e.to_string())
    }

    /// Returns the latest version of the local registry.
    pub fn local_latest_version() -> RegistryVersion {
        D::with_local_registry(|local_registry| local_registry.keys().map(|k| k.version).max().unwrap_or(ZERO_REGISTRY_VERSION))
    }

    /// Syncs the local registry with the remote registry.
    ///
    /// This function will keep fetching registry deltas from the remote registry until the local registry is up to date.
    pub async fn sync_registry_stored<R: RegistryCanisterClient>(client: &R) -> anyhow::Result<()> {
        let mut current_local_version = Self::local_latest_version().get();

        loop {
            let remote_latest_version = ic_nns_common::registry::get_latest_version().await;

            match current_local_version.cmp(&remote_latest_version) {
                Ordering::Less => {
                    ic_cdk::println!("Registry version local {} < remote {}", current_local_version, remote_latest_version);
                }
                Ordering::Equal => {
                    ic_cdk::println!("Local Registry version {} is up to date", current_local_version);
                    break;
                }
                Ordering::Greater => {
                    return Err(anyhow!(
                        "Registry version local {} > remote {}, this should never happen",
                        current_local_version,
                        remote_latest_version
                    ));
                }
            }

            let remote_deltas = client.registry_changes_since(current_local_version).await?;

            // Update the local version to the latest remote version for this iteration.
            current_local_version = remote_deltas
                .iter()
                .flat_map(|delta| delta.values.iter().map(|v| v.version))
                .max()
                .unwrap_or(current_local_version);

            Self::add_deltas(remote_deltas)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
