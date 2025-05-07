use async_trait::async_trait;
use ic_base_types::{CanisterId, PrincipalId, RegistryVersion};
use ic_cdk::call::{Call, CallErrorExt, CallFailed};
use ic_nervous_system_canisters::registry::Registry;
use ic_nervous_system_common::NervousSystemError;
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, deserialize_get_latest_version_response, serialize_get_changes_since_request};
use std::future::Future;

// TODO: Remove this when the RegistryCanister in the IC repo uses ic-cdk 0.18.0

pub struct RegistryCanister {
    canister_id: CanisterId,
}

impl RegistryCanister {
    pub fn new() -> Self {
        Self {
            canister_id: REGISTRY_CANISTER_ID,
        }
    }

    async fn execute_with_retries<F, Fut, Response>(&self, max_attempts: u8, call: F) -> Result<Response, NervousSystemError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<Response, CallFailed>>,
    {
        for _ in 0..max_attempts {
            match call().await {
                Ok(response) => return Ok(response),
                Err(e) if e.is_immediately_retryable() => {
                    continue;
                }
                Err(e) => {
                    return Err(NervousSystemError::new_with_message(format!("Request failed with error: {e}")));
                }
            }
        }

        Err(NervousSystemError::new_with_message(format!(
            "Request failed after {max_attempts} attempts"
        )))
    }
}
#[async_trait]
impl Registry for RegistryCanister {
    async fn get_latest_version(&self) -> Result<RegistryVersion, NervousSystemError> {
        self.execute_with_retries(5, || async {
            Call::bounded_wait(PrincipalId::from(self.canister_id).into(), "get_latest_version")
                .with_raw_args(&[])
                .await
        })
        .await
        .and_then(|response| {
            deserialize_get_latest_version_response(response.into_bytes())
                .map_err(|e| NervousSystemError::new_with_message(format!("Could not decode response {e:?}")))
                .map(RegistryVersion::new)
        })
    }

    async fn registry_changes_since(&self, version: RegistryVersion) -> Result<Vec<RegistryDelta>, NervousSystemError> {
        let bytes = serialize_get_changes_since_request(version.get()).map_err(|e| {
            NervousSystemError::new_with_message(format!("Could not encode request for get_changes_since for version {:?}: {}", version, e))
        })?;

        self.execute_with_retries(5, || async {
            Call::bounded_wait(PrincipalId::from(self.canister_id).into(), "get_changes_since")
                .with_raw_args(&bytes)
                .await
        })
        .await
        .and_then(|response| {
            deserialize_get_changes_since_response(response.into_bytes())
                .map_err(|e| NervousSystemError::new_with_message(format!("Could not decode response {e:?}")))
                .map(|(deltas, _)| deltas)
        })
    }
}
