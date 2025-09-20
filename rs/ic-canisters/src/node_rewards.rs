use std::str::FromStr;

use candid::Principal;
use ic_node_rewards_canister_api::provider_rewards_calculation::{GetNodeProviderRewardsCalculationRequest, GetNodeProviderRewardsCalculationResponse, DailyResults, DayUtc};
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

    pub async fn get_rewards_daily(&self, day: &DayUtc) -> anyhow::Result<DailyResults> {
        self.agent
            .query::<GetNodeProviderRewardsCalculationResponse>(
                &Principal::from_str(NODE_METRICS_CANISTER).map_err(anyhow::Error::from)?,
                "get_node_provider_rewards_calculation",
                candid::encode_one(GetNodeProviderRewardsCalculationRequest {
                    day_timestamp_nanos: day.value.unwrap()
                })?,
            )
            .await?
            .map_err(|e| anyhow::anyhow!(e))
    }
}
