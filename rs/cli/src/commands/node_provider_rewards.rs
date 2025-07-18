use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime};
use clap::Args;
use ic_canisters::node_provider_rewards::NodeProviderRewardsCanisterWrapper;
use ic_types::PrincipalId;
use log::info;
use node_provider_rewards_api::endpoints::{NodeProviderRewardsCalculationArgs, RewardPeriodArgs, RewardsCalculatorResultsV1};
use tabled::{Table, Tabled};

use crate::{auth::AuthRequirement, exe::ExecutableCommand};

#[derive(Args, Debug)]
pub struct NodeProviderRewards {
    /// The id of the node provider
    #[clap(long)]
    pub provider_id: String,

    /// The start date of the rewards period in YYYY-MM-DD format
    #[clap(long)]
    pub start_date: String,

    /// The end date of the rewards period in YYYY-MM-DD format
    #[clap(long)]
    pub end_date: String,
}

#[derive(Tabled)]
struct NodeRewardRow {
    #[tabled(rename = "Node ID")]
    node_id: String,
    #[tabled(rename = "Uptime")]
    uptime: String,
    #[tabled(rename = "Reward (XDR)")]
    reward: String,
}

impl ExecutableCommand for NodeProviderRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        info!("Started action...");

        let start_date = NaiveDate::from_str(&self.start_date)?;
        let end_date = NaiveDate::from_str(&self.end_date)?;
        let start_ts = start_date.and_hms_opt(0, 0, 0).unwrap().timestamp_nanos_opt().unwrap() as u64;
        let end_ts = end_date.and_hms_opt(0, 0, 0).unwrap().timestamp_nanos_opt().unwrap() as u64;

        let args = NodeProviderRewardsCalculationArgs {
            provider_id: PrincipalId::from_str(&self.provider_id)?,
            reward_period: RewardPeriodArgs { start_ts, end_ts },
        };

        let rewards_canister = NodeProviderRewardsCanisterWrapper::new(canister_agent);
        let result: Result<RewardsCalculatorResultsV1, String> = rewards_canister.get_node_provider_rewards_calculation_v1(args).await;

        match result {
            Ok(rewards) => {
                let mut rows = vec![];
                for (node_id, node_reward) in rewards.rewards_per_node {
                    rows.push(NodeRewardRow {
                        node_id: node_id.to_string(),
                        uptime: format!("{:.2}%", node_reward.uptime_percentage * 100.0),
                        reward: format!("{:.2}", node_reward.reward_e8s as f64 / 100_000_000.0),
                    });
                }
                let table = Table::new(rows);
                println!("{}", table);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }

        Ok(())
    }

    fn validate(&self, _args: &crate::exe::args::GlobalArgs, _cmd: &mut clap::Command) {}
}
