use chrono::{DateTime, Datelike};
use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::RewardsCalculationAlgorithmVersion;
use log::info;
use std::collections::BTreeMap;

use crate::auth::AuthRequirement;
use crate::exe::ExecutableCommand;
use crate::exe::args::GlobalArgs;

use super::common::{
    CommonArgs, NodeRewardsConsoleOutput, NodeRewardsCsvOutput, NodeRewardsCtx, NodeRewardsDataFetcher, compute_governance_providers_rewards,
    execute_node_rewards,
};

/// Show past rewards for a given month (YYYY-MM) and compare with governance
#[derive(Args, Debug)]
pub struct PastRewards {
    /// Month in format YYYY-MM
    pub month: String,

    #[clap(flatten)]
    pub common: CommonArgs,
}

impl ExecutableCommand for PastRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        info!("Started action...");

        let gov_rewards_list = governance_client.list_node_provider_rewards(None).await?;

        let target = chrono::NaiveDate::parse_from_str(&(self.month.to_string() + "-01"), "%Y-%m-%d")?;
        let mut idx_in_month: Option<usize> = None;
        for (i, snap) in gov_rewards_list.iter().enumerate() {
            let dt = DateTime::from_timestamp(snap.timestamp as i64, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid governance timestamp"))?
                .date_naive();
            if dt.year() == target.year() && dt.month() == target.month() {
                idx_in_month = Some(i);
                break;
            }
        }
        let i = idx_in_month.ok_or_else(|| anyhow::anyhow!("No governance snapshot found for {}", self.month))?;
        let last = &gov_rewards_list[i];
        let xdr_permyriad_per_icp = last
            .xdr_conversion_rate
            .as_ref()
            .and_then(|x| x.xdr_permyriad_per_icp)
            .ok_or_else(|| anyhow::anyhow!("Missing XDR conversion rate"))?;

        let governance_providers_rewards: BTreeMap<_, _> = compute_governance_providers_rewards(&last.rewards, xdr_permyriad_per_icp);

        let prev = gov_rewards_list
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("Previous governance snapshot not found for {}", self.month))?;

        let start_date = DateTime::from_timestamp(prev.timestamp as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid previous timestamp"))?
            .date_naive();
        let end_date = DateTime::from_timestamp(last.timestamp as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid last timestamp"))?
            .date_naive()
            .pred_opt()
            .ok_or_else(|| anyhow::anyhow!("Cannot get previous day"))?;

        let node_providers = governance_client.get_node_providers().await?;
        let algorithm_version = last.algorithm_version.map(|v| RewardsCalculationAlgorithmVersion { version: v });

        let rewards_ctx = NodeRewardsCtx {
            start_date,
            end_date,
            algorithm_version,
            csv_detailed_output_path: self.common.csv_detailed_output_path.clone(),
            governance_providers_rewards,
            compare_with_governance: self.common.compare_with_governance,
            governance_rewards_raw: last.clone(),
            xdr_permyriad_per_icp,
            node_providers,
            is_past_rewards_mode: true,
        };

        // Create a helper struct that holds the context for trait implementations
        let executor = PastRewardsExecutor { ctx: rewards_ctx };

        let nrc_data = executor.fetch_nrc_data(&node_rewards_client).await?;
        execute_node_rewards(&executor, nrc_data).await
    }
}

/// Helper struct that holds the context for trait implementations
struct PastRewardsExecutor {
    ctx: NodeRewardsCtx,
}

impl NodeRewardsDataFetcher for PastRewardsExecutor {
    fn ctx(&self) -> &NodeRewardsCtx {
        &self.ctx
    }
}

impl NodeRewardsConsoleOutput for PastRewardsExecutor {
    fn ctx(&self) -> &NodeRewardsCtx {
        &self.ctx
    }
}

impl NodeRewardsCsvOutput for PastRewardsExecutor {
    fn ctx(&self) -> &NodeRewardsCtx {
        &self.ctx
    }
}
