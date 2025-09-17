use std::str::FromStr;

use candid::Principal;
use ic_base_types::PrincipalId;
use ic_node_rewards_canister_api::provider_rewards_calculation::{GetNodeProviderRewardsCalculationRequest, GetNodeProviderRewardsCalculationResponse, NodeProviderRewardsDaily};
use rewards_calculation::types::DayUtc;
use crate::IcAgentCanisterClient;

const NODE_METRICS_CANISTER: &str = "uuew5-iiaaa-aaaaa-qbx4q-cai";

pub struct NodeRewardsCanisterWrapper {
    agent: IcAgentCanisterClient,
}

impl From<IcAgentCanisterClient> for NodeRewardsCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        NodeRewardsCanisterWrapper::new(value)
    }
}

impl NodeRewardsCanisterWrapper {
    pub fn new(agent: IcAgentCanisterClient) -> Self {
        Self { agent }
    }

    pub async fn get_provider_rewards_daily(&self, provider_id: PrincipalId, from_day: DayUtc, to_day: DayUtc) -> anyhow::Result<Vec<NodeProviderRewardsDaily>> {
        self.agent
            .query::<GetNodeProviderRewardsCalculationResponse>(
                &Principal::from_str(NODE_METRICS_CANISTER).map_err(anyhow::Error::from)?,
                "get_node_provider_rewards_calculation",
                candid::encode_one(GetNodeProviderRewardsCalculationRequest {
                    from_day_timestamp_nanos: from_day.unix_ts_at_day_start_nanoseconds(),
                    to_day_timestamp_nanos: to_day.unix_ts_at_day_start_nanoseconds(),
                    provider_id: provider_id.0,
                })?,
            )
            .await?
            .map_err(|e| anyhow::anyhow!(e))
    }
}
