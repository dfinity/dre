use anyhow::anyhow;
use chrono::DateTime;
use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use log::info;
use std::collections::BTreeMap;

use crate::auth::AuthRequirement;
use crate::exe::ExecutableCommand;
use crate::exe::args::GlobalArgs;

use super::common::{
    CommonArgs, NodeRewardsConsoleOutput, NodeRewardsCsvOutput, NodeRewardsCtx, NodeRewardsDataFetcher, compute_governance_providers_rewards,
    execute_node_rewards,
};

/// Show ongoing rewards from the latest governance snapshot timestamp to yesterday
#[derive(Args, Debug)]
pub struct Ongoing {
    #[clap(flatten)]
    pub common: CommonArgs,
}

impl ExecutableCommand for Ongoing {
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
        let last_rewards = gov_rewards_list.into_iter().next().unwrap();

        // Range: from latest governance ts to yesterday
        let today = chrono::Utc::now().date_naive();
        let yesterday = today.pred_opt().unwrap();
        let end_ts = yesterday
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid yesterday midnight"))?
            .and_utc()
            .timestamp();
        let start_date = DateTime::from_timestamp(last_rewards.timestamp as i64, 0).unwrap().date_naive();
        let end_date = DateTime::from_timestamp(end_ts, 0).unwrap().date_naive();

        if end_date < start_date {
            return Err(anyhow!("Rewards have been distributed today, wait until tomorrow and retry"));
        }

        let last = governance_client.get_node_provider_rewards().await?;
        let xdr_permyriad_per_icp = last
            .xdr_conversion_rate
            .as_ref()
            .and_then(|x| x.xdr_permyriad_per_icp)
            .ok_or_else(|| anyhow::anyhow!("Missing XDR conversion rate"))?;

        println!("Start date: {:?}, End date: {:?}", last.start_date, last.end_date);

        let governance_providers_rewards: BTreeMap<_, _> = compute_governance_providers_rewards(&last.rewards, xdr_permyriad_per_icp);

        let node_providers = governance_client.get_node_providers().await?;

        let rewards_ctx = NodeRewardsCtx {
            start_date,
            end_date,
            algorithm_version: None,
            csv_detailed_output_path: self.common.csv_detailed_output_path.clone(),
            governance_providers_rewards,
            compare_with_governance: self.common.compare_with_governance,
            governance_rewards_raw: last.clone(),
            xdr_permyriad_per_icp,
            node_providers,
            is_past_rewards_mode: false,
        };

        // Create a helper struct that holds the context for trait implementations
        let executor = OngoingExecutor { ctx: rewards_ctx };

        let nrc_data = executor.fetch_nrc_data(&node_rewards_client).await?;
        execute_node_rewards(&executor, nrc_data).await
    }
}

/// Helper struct that holds the context for trait implementations
struct OngoingExecutor {
    ctx: NodeRewardsCtx,
}

impl NodeRewardsDataFetcher for OngoingExecutor {
    fn ctx(&self) -> &NodeRewardsCtx {
        &self.ctx
    }
}

impl NodeRewardsConsoleOutput for OngoingExecutor {
    fn ctx(&self) -> &NodeRewardsCtx {
        &self.ctx
    }
}

impl NodeRewardsCsvOutput for OngoingExecutor {
    fn ctx(&self) -> &NodeRewardsCtx {
        &self.ctx
    }
}
