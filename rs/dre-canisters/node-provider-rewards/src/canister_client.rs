use crate::metrics::ManagementCanisterClient;
use async_trait::async_trait;
use candid::Principal;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types_private::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};

/// Used to interact with remote IC canisters.
pub struct ICCanisterClient;

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
