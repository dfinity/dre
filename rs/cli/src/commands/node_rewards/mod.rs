use crate::commands::node_rewards::csv_generator::CsvGenerator;
use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};
use chrono::NaiveDate;
use clap::{Args, Subcommand};
use futures_util::future::join_all;
use ic_base_types::PrincipalId;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeProviderRewards, DailyResults};
use ic_node_rewards_canister_api::DateUtc;
use log::info;
use std::collections::BTreeMap;
use tabled::{
    settings::{object::Rows, Alignment, Modify, Style, Width},
    Table, Tabled,
};

mod csv_generator;
mod ongoing;
mod past_rewards;

#[derive(Subcommand, Debug, Clone)]
pub enum NodeRewardsMode {
    /// Show ongoing rewards from the latest governance snapshot timestamp to yesterday
    Ongoing {
        /// If set, write detailed CSVs to this directory
        #[arg(long)]
        csv_detailed_output_path: Option<String>,

        /// Filter to a single provider (full principal or provider prefix)
        #[arg(long)]
        provider_id: Option<String>,
    },
    /// Show past rewards for a given month (YYYY-MM) and compare with governance
    PastRewards {
        /// Month in format YYYY-MM
        month: String,
        /// If set, write detailed CSVs to this directory
        #[arg(long)]
        csv_detailed_output_path: Option<String>,
        /// Filter to a single provider (full principal or provider prefix)
        #[arg(long)]
        provider_id: Option<String>,
    },
}
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
    nrc_xdr_permyriad: u64,
    governance_xdr_permyriad: u64,
    difference_xdr_permyriad: i64,
    underperforming_nodes: Vec<String>,
    daily_rewards: Vec<(DateUtc, DailyNodeProviderRewards)>,
}

#[derive(Args, Debug)]
pub struct NodeRewards {
    /// Subcommand mode: ongoing or past-rewards <month>
    #[command(subcommand)]
    pub mode: NodeRewardsMode,
}

impl NodeRewards {
    /// Get provider prefix from full provider ID
    fn get_provider_prefix(provider_id: &str) -> &str {
        provider_id.split('-').next().unwrap()
    }

    /// Format DateUtc without the " UTC" suffix
    fn format_date_utc(date: DateUtc) -> String {
        let date_str = date.to_string();
        date_str.strip_suffix(" UTC").unwrap().to_string()
    }

    /// Collect underperforming nodes for a provider
    fn collect_underperforming_nodes(&self, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Vec<String> {
        let mut underperforming_nodes = Vec::new();
        for (_, rewards) in daily_rewards {
            for node_result in &rewards.daily_nodes_rewards {
                let multiplier = node_result.performance_multiplier.unwrap();
                if multiplier < 1.0 {
                    let node_id_str = node_result.node_id.unwrap().to_string();
                    let node_prefix = node_id_str.split('-').next().unwrap().to_string();
                    underperforming_nodes.push(node_prefix);
                }
            }
        }
        underperforming_nodes.sort();
        underperforming_nodes.dedup();
        underperforming_nodes
    }

    /// Display the comparison table
    async fn display_comparison_table(&self, provider_data: &[ProviderData]) -> anyhow::Result<()> {
        // Create table data
        let mut table_data = Vec::new();
        for provider in provider_data {
            let provider_id_str = provider.provider_id.to_string();
            let provider_prefix = Self::get_provider_prefix(&provider_id_str);

            // Calculate percentage difference always relative to NRC, always display in XDRPermyriad
            let (diff_value, base_value) = (provider.difference_xdr_permyriad, provider.nrc_xdr_permyriad);
            let percent_diff = if base_value > 0 {
                diff_value as f64 / base_value as f64 * 100.0
            } else {
                0.0
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

            // Always display in XDRPermyriad
            let (nrc_display, governance_display, difference_display) = (
                provider.nrc_xdr_permyriad.to_string(),
                provider.governance_xdr_permyriad.to_string(),
                provider.difference_xdr_permyriad.to_string(),
            );

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
        println!("\n=== NODE REWARDS COMPARISON: NRC vs GOVERNANCE ===");
        println!("Unit: XDRPermyriad | Sorted by decreasing percentage difference");
        println!("\nLegend:");
        println!("• NRC: Node Rewards Canister rewards (XDRPermyriad)");
        println!("• Governance: Governance rewards from NNS (XDRPermyriad)");
        println!("• Difference: NRC - Governance (XDRPermyriad)");
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

        // Run the selected subcommand
        let (start_day, end_day, mut provider_data, subnets_fr_data) = match &self.mode {
            NodeRewardsMode::Ongoing { .. } => ongoing::run(canister_agent.clone(), self).await?,
            NodeRewardsMode::PastRewards { month, .. } => past_rewards::run(canister_agent.clone(), self, month).await?,
        };

        // Resolve subcommand options
        let (csv_path_opt, provider_filter_opt, show_comparison) = match &self.mode {
            NodeRewardsMode::Ongoing {
                csv_detailed_output_path: csv_detailed_output,
                provider_id,
            } => (csv_detailed_output.as_ref(), provider_id.as_ref(), false),
            NodeRewardsMode::PastRewards {
                csv_detailed_output_path: csv_detailed_output,
                provider_id,
                ..
            } => (csv_detailed_output.as_ref(), provider_id.as_ref(), true),
        };

        // Apply provider filter if any (match full principal or provider prefix)
        if let Some(filter) = provider_filter_opt {
            provider_data.retain(|p| {
                let full = p.provider_id.to_string();
                let prefix = Self::get_provider_prefix(&full);
                full == *filter || prefix == filter
            });
        }

        if let Some(output_dir) = csv_path_opt {
            let provider_csv_data: Vec<(PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>)> = provider_data
                .iter()
                .map(|provider| (provider.provider_id, provider.daily_rewards.clone()))
                .collect();
            self.generate_csv_files_by_provider(&provider_csv_data, output_dir, &subnets_fr_data, start_day, end_day)
                .await?;
        } else {
            // Print rewards_summary-like view to console
            self.print_rewards_summary_console(&provider_data)?;
        }

        if show_comparison {
            self.display_comparison_table(&provider_data).await?;
        }

        Ok(())
    }
}

// ================================================================================================
// Shared data fetching and aggregation
// ================================================================================================
async fn fetch_and_aggregate(
    node_rewards_client: &NodeRewardsCanisterWrapper,
    start_day: NaiveDate,
    end_day: NaiveDate,
    xdr_permyriad_per_icp: u64,
    mut gov_rewards_map: BTreeMap<PrincipalId, u64>,
    collect_underperf: impl Fn(&[(DateUtc, DailyNodeProviderRewards)]) -> Vec<String>,
) -> anyhow::Result<(NaiveDate, NaiveDate, Vec<ProviderData>, Vec<(DateUtc, String, f64)>)> {
    println!("Fetching node rewards for all providers from NRC from {} to {}...", start_day, end_day);

    let days: Vec<DateUtc> = start_day.iter_days().take_while(|day| day <= &end_day).map(DateUtc::from).collect();
    let responses: Vec<anyhow::Result<DailyResults>> =
        join_all(days.iter().map(|day| async move { node_rewards_client.get_rewards_daily(*day).await })).await;

    let mut provider_results = BTreeMap::new();
    let mut subnets_fr_data = Vec::new();

    for (day, response) in days.into_iter().zip(responses.into_iter()) {
        match response {
            Ok(daily_results) => {
                for (provider_id, provider_rewards) in daily_results.provider_results {
                    let rewards = provider_results.entry(provider_id).or_insert_with(Vec::new);
                    rewards.push((day, provider_rewards));
                }
                // Collect subnets failure rates
                for (subnet_id, failure_rate) in daily_results.subnets_failure_rate {
                    subnets_fr_data.push((day, subnet_id.to_string(), failure_rate));
                }
            }
            Err(e) => {
                println!("Error fetching node rewards for provider: {}", e);
            }
        }
    }

    let mut provider_daily_data = Vec::new();
    for (provider_id, daily_rewards) in provider_results {
        let nrc_xdr_permyriad: u64 = daily_rewards.iter().map(|(_, reward)| reward.rewards_total_xdr_permyriad.unwrap()).sum();
        let principal: PrincipalId = provider_id.to_string().parse().unwrap();

        let governance_icp = gov_rewards_map.remove(&principal).unwrap() / 100_000_000;
        let governance_xdr_permyriad = governance_icp * xdr_permyriad_per_icp;
        let nrc_xdr_permyriad_decimal = nrc_xdr_permyriad;
        let difference_xdr_permyriad = (nrc_xdr_permyriad_decimal as i64) - (governance_xdr_permyriad as i64);
        let underperforming_nodes = collect_underperf(&daily_rewards);

        provider_daily_data.push(ProviderData {
            provider_id: principal,
            nrc_xdr_permyriad: nrc_xdr_permyriad_decimal,
            governance_xdr_permyriad,
            difference_xdr_permyriad,
            underperforming_nodes,
            daily_rewards,
        });
    }

    provider_daily_data.sort_by(|a, b| {
        let a_percent = if a.nrc_xdr_permyriad > 0 {
            a.difference_xdr_permyriad as f64 / a.nrc_xdr_permyriad as f64 * 100.0
        } else {
            0.0
        };
        let b_percent = if b.nrc_xdr_permyriad > 0 {
            b.difference_xdr_permyriad as f64 / b.nrc_xdr_permyriad as f64 * 100.0
        } else {
            0.0
        };
        b_percent.partial_cmp(&a_percent).unwrap()
    });

    Ok((start_day, end_day, provider_daily_data, subnets_fr_data))
}

impl NodeRewards {
    fn print_rewards_summary_console(&self, provider_data: &[ProviderData]) -> anyhow::Result<()> {
        use tabled::settings::{object::Rows, Alignment, Modify, Style, Width};
        use tabled::{Table, Tabled};

        #[derive(Tabled)]
        struct DailyRewardSummary {
            #[tabled(rename = "Day")]
            day: String,
            #[tabled(rename = "Base Rewards Total")]
            base_rewards_total: String,
            #[tabled(rename = "Adjusted Rewards Total")]
            adjusted_rewards_total: String,
            #[tabled(rename = "Adjusted Rewards %")]
            adjusted_rewards_percent: String,
            #[tabled(rename = "Nodes")]
            nodes: usize,
            #[tabled(rename = "Assigned")]
            assigned_count: usize,
            #[tabled(rename = "Underperf")]
            underperf_count: usize,
            #[tabled(rename = "Underperf Nodes")]
            underperf_nodes: String,
        }

        println!("\n=== DAILY REWARDS SUMMARY ===");
        println!("Unit: XDRPermyriad per day");
        println!("\nLegend:");
        println!("• Day: UTC day (YYYY-MM-DD)");
        println!("• Base Rewards Total: Sum of base_rewards_xdr_permyriad across all nodes on that day");
        println!("• Adjusted Rewards Total: Sum of adjusted_rewards_xdr_permyriad across all nodes on that day");
        println!("• Adjusted Rewards %: (Adjusted Rewards Total / Base Rewards Total) × 100%");
        println!("• Nodes: Nodes found in registry on that day");
        println!("• Assigned: Nodes assigned to a subnet on that day");
        println!("• Underperf: Nodes with performance multiplier < 1 on that day");
        println!("• Underperf Nodes: Comma-separated underperforming node IDs (prefixes)");
        println!();

        for provider in provider_data {
            let provider_id_str = provider.provider_id.to_string();
            let provider_prefix = Self::get_provider_prefix(&provider_id_str);
            println!("\n=== Provider: {} ===", provider_prefix);

            let mut table_data = Vec::new();
            for (day, rewards) in &provider.daily_rewards {
                let day_str = Self::format_date_utc(*day);
                let nodes_in_registry = rewards.daily_nodes_rewards.len();

                // Sum base and adjusted rewards across all nodes for the day
                let base_rewards_total: u64 = rewards.daily_nodes_rewards.iter().map(|n| n.base_rewards_xdr_permyriad.unwrap()).sum();

                let adjusted_rewards_total: u64 = rewards
                    .daily_nodes_rewards
                    .iter()
                    .map(|n| n.adjusted_rewards_xdr_permyriad.unwrap())
                    .sum();

                // Calculate adjusted rewards percentage
                let adjusted_rewards_percent = if base_rewards_total > 0 {
                    format!("{:.2}%", (adjusted_rewards_total as f64 / base_rewards_total as f64) * 100.0)
                } else {
                    "N/A".to_string()
                };

                // Count assigned nodes
                let assigned_count = rewards
                    .daily_nodes_rewards
                    .iter()
                    .filter(|node_result| {
                        matches!(
                            node_result.daily_node_failure_rate,
                            Some(ic_node_rewards_canister_api::provider_rewards_calculation::DailyNodeFailureRate::SubnetMember { .. })
                        )
                    })
                    .count();

                let mut underperf_prefixes: Vec<String> = rewards
                    .daily_nodes_rewards
                    .iter()
                    .filter(|node_result| node_result.performance_multiplier.unwrap() < 1.0)
                    .map(|node_result| {
                        let node_id_str = node_result.node_id.unwrap().to_string();
                        node_id_str.split('-').next().unwrap().to_string()
                    })
                    .collect();
                underperf_prefixes.sort();
                underperf_prefixes.dedup();
                let underperforming_nodes_count = underperf_prefixes.len();
                let underperforming_nodes = if underperf_prefixes.is_empty() {
                    "None".to_string()
                } else {
                    let nodes_str = underperf_prefixes.join(", ");
                    if nodes_str.len() > 30 {
                        format!("{}...", &nodes_str[..27])
                    } else {
                        nodes_str
                    }
                };

                table_data.push(DailyRewardSummary {
                    day: day_str,
                    base_rewards_total: base_rewards_total.to_string(),
                    adjusted_rewards_total: adjusted_rewards_total.to_string(),
                    adjusted_rewards_percent,
                    nodes: nodes_in_registry,
                    assigned_count,
                    underperf_count: underperforming_nodes_count,
                    underperf_nodes: underperforming_nodes,
                });
            }

            let mut table = Table::new(table_data);
            table
                .with(Style::ascii())
                .with(Modify::new(Rows::new(0..1)).with(Alignment::center()))
                .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
                .with(Width::wrap(250).keep_words(true));

            println!("{}", table);
        }
        Ok(())
    }
}
