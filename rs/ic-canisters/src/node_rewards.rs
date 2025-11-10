use std::cell::RefCell;
use std::str::FromStr;

use crate::IcAgentCanisterClient;
use candid::Principal;
use ic_node_rewards_canister_api::provider_rewards_calculation::{
    DailyResults, DateUtc, GetNodeProvidersRewardsCalculationRequest, GetNodeProvidersRewardsCalculationResponse,
};

const NODE_METRICS_CANISTER: &str = "sgymv-uiaaa-aaaaa-aaaia-cai";
const NODE_METRICS_CANISTER_DEV: &str = "uuew5-iiaaa-aaaaa-qbx4q-cai";

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
        Self {
            agent,
        }
    }

    pub async fn get_rewards_daily(&self, day: DateUtc) -> anyhow::Result<DailyResults> {
        self.agent
            .query::<GetNodeProvidersRewardsCalculationResponse>(
                &Principal::from_str(NODE_METRICS_CANISTER_DEV).map_err(anyhow::Error::from)?,
                "get_node_providers_rewards_calculation",
                candid::encode_one(GetNodeProvidersRewardsCalculationRequest { day })?,
            )
            .await?
            .map_err(|e| anyhow::anyhow!(e))
    }
}
