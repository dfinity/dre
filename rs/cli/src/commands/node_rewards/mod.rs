use std::collections::BTreeMap;
use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};
use clap::Args;
use futures_util::future::join_all;
use ic_base_types::PrincipalId;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use log::info;
use rewards_calculation::performance_based_algorithm::results::{DailyResults, NodeProviderRewards};
use rewards_calculation::types::DayUtc;
use rust_decimal::Decimal;
use tabled::{
    settings::{object::Rows, Alignment, Modify, Style, Width},
    Table, Tabled,
};
use crate::commands::node_rewards::csv_trait::CsvGenerator;

mod csv_trait;
#[derive(Tabled)]
struct ProviderComparison {
    #[tabled(rename = "Provider")]
    provider: String,
    #[tabled(rename = "NRC")]
    nrc_rewards: String,
    #[tabled(rename = "Governance")]
    governance_rewards: String,
    #[tabled(rename = "Difference")]
    difference: String,
    #[tabled(rename = "% Diff")]
    percent_diff: String,
    #[tabled(rename = "Underperforming Nodes")]
    underperforming_nodes: String,
}

struct ProviderData {
    provider_id: PrincipalId,
    nrc_icp: Decimal,
    governance_icp: Decimal,
    nrc_xdr_permyriad: Decimal,
    governance_xdr_permyriad: Decimal,
    difference: Decimal,
    underperforming_nodes: Vec<String>,
    daily_rewards: Vec<NodeProviderRewards>,
}

#[derive(Args, Debug)]
pub struct NodeRewards {
    /// Optional path to generate CSV files. If not provided, only console output will be shown.
    #[arg(long)]
    pub csv_output_path: Option<String>,

    /// Display results in XDRPermyriad instead of ICP
    #[arg(long)]
    pub xdr_permyriad: bool,
}

impl NodeRewards {
    /// Get provider prefix from full provider ID
    fn get_provider_prefix(provider_id: &str) -> &str {
        provider_id.split('-').next().unwrap_or(provider_id)
    }

    async fn _execute(&self, canister_agent: ic_canisters::IcAgentCanisterClient) -> anyhow::Result<Vec<ProviderData>, anyhow::Error> {
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        let mut gov_rewards = governance_client.list_node_provider_rewards(None).await?.into_iter();
        let last_rewards = gov_rewards.next().unwrap();
        let mut gov_rewards_map = last_rewards
            .rewards
            .into_iter()
            .map(|rewards| (rewards.node_provider.unwrap().id.unwrap(), rewards.amount_e8s))
            .collect::<BTreeMap<_, _>>();

        // Fetch node rewards for all providers from NRC concurrently
        let month_before_ts = gov_rewards.next().unwrap().timestamp;
        let start_day = DayUtc::from_secs(month_before_ts);
        let end_day = DayUtc::from_secs(last_rewards.timestamp).previous_day();

        let xdr_permyriad_per_icp: Decimal = last_rewards.xdr_conversion_rate.unwrap().xdr_permyriad_per_icp.unwrap().into();
        let node_rewards_client_ref = &node_rewards_client;
        println!("Fetching node rewards for all providers from NRC from {} to {}...", start_day, end_day);

        let responses: Vec<anyhow::Result<DailyResults>> = join_all(start_day.days_until(&end_day).unwrap().into_iter().map(|day| async move {
            node_rewards_client_ref.get_rewards_daily(&day.into()).await.map( |result| result.into())
        })).await;

        let mut provider_results = BTreeMap::new();

        for response in responses {
            match response {
                Ok(daily_results) => {
                    for (provider_id, daily_rewards) in daily_results.provider_results {
                        let rewards = provider_results.entry(provider_id).or_insert_with(Vec::new);
                        rewards.push(daily_rewards);
                    }
                }
                Err(e) => {
                    println!("Error fetching node rewards for provider: {}", e);
                }
            }
        }

        let mut provider_daily_data = Vec::new();


        for (provider_id, daily_rewards) in provider_results {
            let nrc_xdr_permyriad = daily_rewards
                .iter()
                .cloned()
                .map(|reward| reward.rewards_total)
                .sum();
            let nrc_icp = nrc_xdr_permyriad / xdr_permyriad_per_icp;
            let principal: PrincipalId = PrincipalId::from(provider_id.0); 

            let governance_icp = Decimal::from(gov_rewards_map.remove(&principal).unwrap()) / Decimal::from(100_000_000); // Convert e8s to ICP
            let governance_xdr_permyriad = governance_icp * xdr_permyriad_per_icp; // Convert ICP to XDRPermyriad
            let difference = nrc_icp - governance_icp;
            let underperforming_nodes = self.collect_underperforming_nodes(&daily_rewards);

            provider_daily_data.push(ProviderData {
                provider_id: principal,
                nrc_xdr_permyriad,
                nrc_icp,
                governance_icp,
                governance_xdr_permyriad,
                difference,
                underperforming_nodes,
                daily_rewards,
            });
        }

        // Sort by decreasing percentage difference
        provider_daily_data.sort_by(|a, b| {
            let a_percent = if a.governance_icp > Decimal::ZERO {
                a.difference / a.governance_icp * Decimal::from(100)
            } else {
                Decimal::ZERO
            };
            let b_percent = if b.governance_icp > Decimal::ZERO {
                b.difference / b.governance_icp * Decimal::from(100)
            } else {
                Decimal::ZERO
            };
            b_percent.partial_cmp(&a_percent).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(provider_daily_data)
    }

    /// Collect underperforming nodes for a provider
    fn collect_underperforming_nodes(&self, daily_rewards: &[NodeProviderRewards]) -> Vec<String> {
        let mut underperforming_nodes = Vec::new();
        for daily_reward in daily_rewards {
            
            daily_reward.
            if let Some(node_provider_rewards) = &daily_reward {
                let node_results = &node_provider_rewards.nodes_results;
                for node_result in node_results {
                    let multiplier = Decimal::try_from(node_result.performance_multiplier_percent.as_ref().unwrap().clone()).unwrap();
                    if multiplier < Decimal::from(1) {
                        if let Some(node_id) = &node_result.node_id {
                            let node_id_str = node_id.to_string();
                            let node_prefix = node_id_str.split('-').next().unwrap_or(&node_id_str);
                            underperforming_nodes.push(node_prefix.to_string());
                        }
                    }
                }
            }
        }
        underperforming_nodes.sort();
        underperforming_nodes.dedup();
        underperforming_nodes
    }

    /// Display the comparison table
    async fn display_comparison_table(&self, provider_data: &[ProviderData], xdr_permyriad: bool) -> anyhow::Result<()> {
        // Create table data
        let mut table_data = Vec::new();
        for provider in provider_data {
            let provider_id_str = provider.provider_id.to_string();
            let provider_prefix = Self::get_provider_prefix(&provider_id_str);

            // Calculate percentage difference
            let base_value = if provider.difference >= Decimal::ZERO {
                provider.governance_icp
            } else {
                provider.nrc_icp
            };
            let percent_diff = if base_value > Decimal::ZERO {
                provider.difference / base_value * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            let underperforming_list = if provider.underperforming_nodes.is_empty() {
                "None".to_string()
            } else {
                let list = provider.underperforming_nodes.join(", ");
                if list.len() > 50 {
                    format!("{}...", &list[..47])
                } else {
                    list
                }
            };

            let (nrc_display, governance_display, difference_display) = if xdr_permyriad {
                (
                    format!("{:.0}", provider.nrc_xdr_permyriad),
                    format!("{:.0}", provider.governance_xdr_permyriad),
                    format!("{:.0}", provider.difference),
                )
            } else {
                (
                    format!("{:.0}", provider.nrc_icp),
                    format!("{:.0}", provider.governance_icp),
                    format!("{:.0}", provider.difference),
                )
            };

            table_data.push(ProviderComparison {
                provider: provider_prefix.to_string(),
                nrc_rewards: nrc_display,
                governance_rewards: governance_display,
                difference: difference_display,
                percent_diff: format!("{:.4}%", percent_diff),
                underperforming_nodes: underperforming_list,
            });
        }

        // Create and display table
        let unit_label = if xdr_permyriad { "XDRPermyriad" } else { "ICP" };
        println!("\n=== NODE REWARDS COMPARISON: NRC vs GOVERNANCE ===");
        println!("Unit: {} | Sorted by decreasing percentage difference", unit_label);
        println!("\nLegend:");
        println!("• NRC: Node Rewards Canister rewards");
        println!("• Governance: Governance rewards from NNS");
        println!("• Difference: NRC - Governance");
        println!("• % Diff: (Difference / Base Value) × 100%");
        println!();

        let mut table = Table::new(table_data);
        table
            .with(Style::modern())
            .with(Modify::new(Rows::new(0..1)).with(Alignment::center()))
            .with(Width::wrap(120).keep_words(true));

        println!("{}", table);

        println!("\n=== SUMMARY ===");
        println!("Successfully processed {} providers", provider_data.len());

        Ok(())
    }
}

impl CsvGenerator for NodeRewards {}

impl ExecutableCommand for NodeRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
        info!("Started action...");

        let provider_data = self._execute(canister_agent).await?;

        if let Some(ref output_dir) = self.csv_output_path {
            // Prepare provider data for CSV generation
            let provider_csv_data: Vec<(PrincipalId, Vec<NodeProviderRewards>)> = provider_data
                .iter()
                .map(|provider| (provider.provider_id, provider.daily_rewards.clone()))
                .collect();

            self.generate_csv_files_by_provider(&provider_csv_data, output_dir).await?;
        }
        self.display_comparison_table(&provider_data, self.xdr_permyriad).await?;

        Ok(())
    }
}
