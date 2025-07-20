use std::str::FromStr;

use crate::{auth::AuthRequirement, exe::ExecutableCommand};
use chrono::{NaiveDate, NaiveDateTime};
use clap::Args;
use ic_canisters::node_provider_rewards::NodeProviderRewardsCanisterWrapper;
use ic_types::PrincipalId;
use log::info;
use node_provider_rewards_api::endpoints::{
    DailyResults, NodeProviderRewardsCalculationArgs, NodeStatus, RewardPeriodArgs, RewardsCalculatorResultsV1,
};
use tabled::builder::Builder;
use tabled::settings::object::Rows;
use tabled::settings::style::LineText;
use tabled::settings::Style;
use tabled::{Table, Tabled};

#[derive(Args, Debug)]
pub struct NodeProviderRewards {
    #[clap(long)]
    pub provider_id: PrincipalId,

    pub start_date: String,

    pub end_date: String,
}

impl ExecutableCommand for NodeProviderRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &crate::exe::args::GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        let args = NodeProviderRewardsCalculationArgs {
            provider_id: self.provider_id,
            reward_period: RewardPeriodArgs::from_dd_mm_yyyy(&self.start_date, &self.end_date)?,
        };
        let npr_canister = NodeProviderRewardsCanisterWrapper::new(canister_agent);
        let result = npr_canister.get_node_provider_rewards_calculation_v1(args).await?;

        for (node_id, node_results) in result.results_by_node {
            let mut builder = Builder::default();

            builder.push_record(["Day UTC", "Status", "Performance Multiplier", "Base Rewards", "Adjusted Rewards"]);

            for (day, results_by_day) in node_results.daily_results {
                let DailyResults {
                    node_status,
                    performance_multiplier,
                    base_rewards,
                    adjusted_rewards,
                    ..
                } = results_by_day;

                let status = match node_status {
                    NodeStatus::Assigned { node_metrics } => format!(
                        "Assigned\nSubnet: {}\nSubnet FR: {:?}\nProposed Blocks: {}\nFailed Blocks: {}\nOriginal FR: {:?}\nRelative FR: {:?}",
                        node_metrics.subnet_assigned.get(),
                        node_metrics.subnet_assigned_fr,
                        node_metrics.num_blocks_proposed,
                        node_metrics.num_blocks_failed,
                        node_metrics.original_fr,
                        node_metrics.relative_fr
                    ),
                    NodeStatus::Unassigned { extrapolated_fr } => format!("Unassigned\nExtrapolated FR: {:?}", extrapolated_fr),
                };

                builder.push_record(vec![
                    day.to_string(),
                    status,
                    performance_multiplier.to_string(),
                    base_rewards.to_string(),
                    adjusted_rewards.to_string(),
                ]);
            }

            let mut table = builder.build();
            table.with(LineText::new(node_id.get().to_string(), Rows::first()).offset(2));
            println!("{}", table);
        }

        Ok(())
    }
}
