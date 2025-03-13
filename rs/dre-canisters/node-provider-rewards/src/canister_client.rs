use crate::metrics::ManagementCanisterClient;
use crate::registry_store::RegistryCanisterClientError::CallError;
use crate::registry_store::{RegistryCanisterClient, RegistryCanisterClientError};
use async_trait::async_trait;
use candid::Principal;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_nns_constants::REGISTRY_CANISTER_ID;
use ic_registry_transport::pb::v1::RegistryDelta;
use ic_registry_transport::{deserialize_get_changes_since_response, serialize_get_changes_since_request};

/// Used to interact with remote IC canisters.
pub struct ICCanisterClient {}

#[async_trait]
impl RegistryCanisterClient for ICCanisterClient {
    async fn registry_changes_since(&self, version: u64) -> Result<Vec<RegistryDelta>, RegistryCanisterClientError> {
        let buff = serialize_get_changes_since_request(version)?;
        let registry_canister_principal = Principal::from(REGISTRY_CANISTER_ID);

        let response = ic_cdk::api::call::call_raw(registry_canister_principal, "get_changes_since", buff, 0)
            .await
            .map_err(|(code, message)| CallError(code as u32, message))?;
        let (registry_delta, _) = deserialize_get_changes_since_response(response)?;
        Ok(registry_delta)
    }
}

#[async_trait]
impl ManagementCanisterClient for ICCanisterClient {
    /// Queries the `node_metrics_history` endpoint of the management canisters of the subnet specified
    /// in the 'contract' to fetch daily node metrics.
    async fn node_metrics_history(&self, contract: NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)> {
        ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
            Principal::management_canister(),
            "node_metrics_history",
            (contract,),
            0_u128,
        )
        .await
    }
}
