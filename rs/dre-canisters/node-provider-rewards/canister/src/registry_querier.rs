use async_trait::async_trait;
use ic_base_types::{CanisterId, PrincipalId, RegistryVersion};
use ic_cdk::call::Call;
use ic_nervous_system_canisters::registry::Registry;
use ic_nervous_system_common::NervousSystemError;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, deserialize_get_latest_version_response, serialize_get_changes_since_request};

pub struct RegistryQuerier {
    canister_id: CanisterId,
}

impl RegistryQuerier {
    pub fn new() -> Self {
        Self {
            canister_id: REGISTRY_CANISTER_ID,
        }
    }
}
#[async_trait]
impl Registry for RegistryQuerier {
    async fn get_latest_version(&self) -> Result<RegistryVersion, NervousSystemError> {
        Call::bounded_wait(PrincipalId::from(self.canister_id).0, "get_latest_version")
            .with_raw_args(&[])
            .await
            .map_err(|err| NervousSystemError::new_with_message(format!("Request to get_latest_version failed with code {}", err)))
            .and_then(|r| {
                deserialize_get_latest_version_response(r.into_bytes())
                    .map_err(|e| NervousSystemError::new_with_message(format!("Could not decode response {e:?}")))
                    .map(RegistryVersion::new)
            })
    }

    async fn registry_changes_since(&self, version: RegistryVersion) -> Result<Vec<RegistryDelta>, NervousSystemError> {
        let bytes = serialize_get_changes_since_request(version.get()).map_err(|e| {
            NervousSystemError::new_with_message(format!("Could not encode request for get_changes_since for version {:?}: {}", version, e))
        })?;

        Call::bounded_wait(PrincipalId::from(self.canister_id).0, "get_changes_since")
            .with_raw_args(&bytes)
            .await
            .map_err(|err| NervousSystemError::new_with_message(format!("Request to get_changes_since failed with code {}", err)))
            .and_then(|r| {
                deserialize_get_changes_since_response(r.into_bytes())
                    .map_err(|e| NervousSystemError::new_with_message(format!("Could not decode response {e:?}")))
                    .map(|(deltas, _)| deltas)
            })
    }
}
