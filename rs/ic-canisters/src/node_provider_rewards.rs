use candid::Principal;
use log::error;
use node_provider_rewards_api::endpoints::{NodeProviderRewardsCalculationArgs, RewardsCalculatorResultsV1};
use std::str::FromStr;

use crate::IcAgentCanisterClient;
const NODE_PROVIDER_REWARDS_CANISTER: &str = "4ofd5-6aaaa-aaaaa-qahza-cai";

pub struct NodeProviderRewardsCanisterWrapper {
    agent: IcAgentCanisterClient,
}

impl From<IcAgentCanisterClient> for NodeProviderRewardsCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        NodeProviderRewardsCanisterWrapper::new(value)
    }
}

impl NodeProviderRewardsCanisterWrapper {
    pub fn new(agent: IcAgentCanisterClient) -> Self {
        Self { agent }
    }

    pub async fn get_node_provider_rewards_calculation_v1(
        &self,
        args: NodeProviderRewardsCalculationArgs,
    ) -> anyhow::Result<RewardsCalculatorResultsV1> {
        self.agent
            .query::<Result<RewardsCalculatorResultsV1, String>>(
                &Principal::from_str(NODE_PROVIDER_REWARDS_CANISTER).map_err(anyhow::Error::from)?,
                "get_node_provider_rewards_calculation_v1",
                candid::encode_one(args)?,
            )
            .await?
            .map_err(|e| {
                error!("Failed to decode RewardsCalculatorResultsV1");
                anyhow::anyhow!(e)
            })
    }
}
