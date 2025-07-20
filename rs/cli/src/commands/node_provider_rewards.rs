use crate::{auth::AuthRequirement, exe::ExecutableCommand};
use clap::Args;
use ic_base_types::NodeId;
use ic_canisters::node_provider_rewards::NodeProviderRewardsCanisterWrapper;
use ic_types::PrincipalId;
use indexmap::IndexMap;
use node_provider_rewards_api::endpoints::{DailyResults, DayUTC, NodeProviderRewardsCalculationArgs, NodeStatus, RewardPeriodArgs, XDRPermyriad};
use tabled::builder::Builder;
use tabled::settings::object::Rows;
use tabled::settings::style::LineText;

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
        let mut overall_performance: IndexMap<DayUTC, (Vec<NodeId>, XDRPermyriad)> = IndexMap::new();

        for (node_id, node_results) in result.results_by_node {
            let mut builder = Builder::default();

            builder.push_record([
                "Day UTC",
                "Status",
                "Subnet FR",
                "Blocks Proposed/Failed",
                "Original FR",
                "FR relative/extrapolated",
                "Performance Multiplier",
                "Base Rewards",
                "Adjusted Rewards",
            ]);

            for (day, results_by_day) in node_results.daily_results {
                let mut record: Vec<String> = vec![day.clone().into()];
                let DailyResults {
                    node_status,
                    performance_multiplier,
                    base_rewards,
                    adjusted_rewards,
                    ..
                } = results_by_day;
                let (underperforming_nodes, rewards_day_total) = overall_performance.entry(day).or_insert((Vec::new(), XDRPermyriad(0.0)));

                match node_status {
                    NodeStatus::Assigned { node_metrics } => {
                        let subnet_prefix = node_metrics.subnet_assigned.get().to_string().split('-').next().unwrap().to_string();
                        record.extend(vec![
                            format!("{} - {}", "Assigned", subnet_prefix),
                            node_metrics.subnet_assigned_fr.to_string(),
                            format!("{}/{}", node_metrics.num_blocks_proposed, node_metrics.num_blocks_failed),
                            node_metrics.original_fr.to_string(),
                            node_metrics.relative_fr.to_string(),
                        ])
                    }
                    NodeStatus::Unassigned { extrapolated_fr } => record.extend(vec![
                        "Unassigned".to_string(),
                        "N/A".to_string(),
                        "N/A".to_string(),
                        "N/A".to_string(),
                        extrapolated_fr.to_string(),
                    ]),
                };

                if performance_multiplier.0 < 1.0 {
                    underperforming_nodes.push(node_id);
                }
                rewards_day_total.0 += adjusted_rewards.0;

                record.extend(vec![
                    performance_multiplier.to_string(),
                    base_rewards.to_string(),
                    adjusted_rewards.to_string(),
                ]);
                builder.push_record(record);
            }

            let mut table = builder.build();
            let node_title = format!("Node ID: {}", node_id.get().to_string());
            table.with(LineText::new(node_title, Rows::first()).offset(2));
            println!("{}", table);
        }

        let mut builder = Builder::default();
        builder.push_record(["Day UTC", "Underperforming Nodes", "Total Daily Rewards"]);
        for (day, (underperforming_nodes, total_rewards)) in overall_performance {
            let node_ids: Vec<String> = underperforming_nodes
                .iter()
                .map(|id| id.get().to_string().split('-').next().unwrap().to_string())
                .collect();
            builder.push_record([day.into(), node_ids.join("\n"), total_rewards.to_string()]);
        }
        let mut table = builder.build();
        let title = format!(
            "Overall Performance for Provider: {} from {} to {}",
            self.provider_id, self.start_date, self.end_date
        );
        table.with(LineText::new(title, Rows::first()).offset(2));
        println!("{}", table);
        Ok(())
    }
}
