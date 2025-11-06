use chrono::Datelike;
use crate::commands::node_rewards::csv_generator::CsvGenerator;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use chrono::{DateTime, NaiveDate};
use clap::{Args, Subcommand};
use futures_util::future::join_all;
use ic_base_types::{PrincipalId, SubnetId};
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::DateUtc;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeProviderRewards, DailyResults};
use log::info;
use std::collections::BTreeMap;
use itertools::Itertools;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Merge, Modify, Style, Width, object::Rows},
};
use ic_canisters::governance::GovernanceCanisterWrapper;

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

struct ProviderRewards {
    provider_id: PrincipalId,
    nrc_total_xdr_permyriad: u64,
    daily_rewards: Vec<(DateUtc, DailyNodeProviderRewards)>,
}

struct SubnetFailureRates {
    subnet_id: SubnetId,
    daily_failure_rates: Vec<(DateUtc, f64)>,
}

#[derive(Args, Debug)]
pub struct NodeRewards {
    /// Subcommand mode: ongoing or past-rewards <month>
    #[command(subcommand)]
    pub mode: NodeRewardsMode,
}

impl NodeRewards {

    /// Format DateUtc without the " UTC" suffix
    fn format_date_utc(date: DateUtc) -> String {
        let date_str = date.to_string();
        date_str.strip_suffix(" UTC").unwrap().to_string()
    }

    /// Display the comparison table
    async fn display_comparison_table(&self, provider_data: &[ProviderRewards]) -> anyhow::Result<()> {
        // Create table data
        let mut table_data = Vec::new();
        for provider in provider_data {
            let provider_id_str = provider.provider_id.to_string();
            let provider_prefix = Self::get_provider_prefix(&provider_id_str);

            // Calculate percentage difference always relative to NRC, always display in XDRPermyriad
            let (diff_value, base_value) = (provider.difference_xdr_permyriad, provider.nrc_total_xdr_permyriad);
            let percent_diff = if base_value > 0 {
                diff_value as f64 / base_value as f64 * 100.0
            } else {
                0.0
            };

            let underperforming_list = if provider.underperforming_nodes.is_empty() {
                "None".to_string()
            } else {
                let list = provider.underperforming_nodes.join(", ");
                if list.len() > 50 { format!("{}...", &list[..47]) } else { list }
            };

            // Always display in XDRPermyriad
            let (nrc_display, governance_display, difference_display) = (
                provider.nrc_total_xdr_permyriad.to_string(),
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
            .with(Width::wrap(120).keep_words(true))
            .with(Merge::vertical());

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

        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        let mut gov_rewards_list = governance_client.list_node_provider_rewards(None).await?;
        // Run the selected subcommand
        let (s)match &self.mode {
            NodeRewardsMode::Ongoing {
                csv_detailed_output_path,
                provider_id,
            } => {
                let last_rewards = gov_rewards_list.into_iter().next().unwrap();
                let start_date = DateTime::from_timestamp(last_rewards.timestamp as i64, 0).unwrap().date_naive();
                let end_date = chrono::Utc::now().date_naive().pred_opt().unwrap();

                (start_date, end_date, provider_id, csv_detailed_output_path)
            },
            NodeRewardsMode::PastRewards { month,                csv_detailed_output_path,
                provider_id } => {
                let target = chrono::NaiveDate::parse_from_str(&(month.to_string() + "-01"), "%Y-%m-%d")?;
                let mut idx_in_month: Option<usize> = None;
                for (i, snap) in gov_rewards_list.iter().enumerate() {
                    let dt = DateTime::from_timestamp(snap.timestamp as i64, 0).ok_or_else(|| anyhow::anyhow!("Invalid governance timestamp"))?.date_naive();
                    if dt.year() == target.year() && dt.month() == target.month() {
                        idx_in_month = Some(i);
                        break;
                    }
                }
                let i = idx_in_month.ok_or_else(|| anyhow::anyhow!("No governance snapshot found for {}", month))?;
                let last = &gov_rewards_list[i];
                let gov_providers_rewards = last
                    .rewards
                    .clone()
                    .into_iter()
                    .map(|r| (r.node_provider.unwrap().id.unwrap(), r.amount_e8s))
                    .collect();

                let prev = gov_rewards_list
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("Previous governance snapshot not found for {}", month))?;

                let start_date = DateTime::from_timestamp(prev.timestamp as i64, 0).unwrap().date_naive();
                let end_date = DateTime::from_timestamp(last.timestamp as i64, 0)
                    .unwrap()
                    .date_naive()
                    .pred_opt()
                    .unwrap();

                (start_date, end_date)

                let (nrc_providers_rewards, subnets_failure_rates) = self.fetch_nrc_rewards(&node_rewards_client, start_day, end_day).await?;



                self.display_comparison_table(&nrc_providers_rewards, gov_providers_rewards).await?;
            },
        }

        let (mut nrc_rewards, subnets_failure_rates) = self.fetch_nrc_rewards(&node_rewards_client, start_day, end_date).await?;

        if let Some(filter) = provider_id {
            nrc_rewards.retain(|p| {
                let full = p.provider_id.to_string();
                let prefix = Self::get_provider_prefix(&full);
                full == *filter || prefix == filter
            });
        }

        if let Some(output_dir) = csv_detailed_output_path {
            let provider_csv_data: Vec<(PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>)> = provider_data
                .iter()
                .map(|provider| (provider.provider_id, provider.daily_rewards.clone()))
                .collect();
            self.generate_csv_files_by_provider(&provider_csv_data, output_dir, &subnets_failure_rates, start_day, end_day)
                .await?;
        } else {
            // Print rewards_summary-like view to console
            self.print_rewards_summary_console(&provider_data)?;
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

        }

        Ok(())
    }
}

impl NodeRewards {

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

    async fn fetch_nrc_rewards(
        &self,
        node_rewards_client: &NodeRewardsCanisterWrapper,
        start_day: NaiveDate,
        end_day: NaiveDate,
    ) -> anyhow::Result<(Vec<ProviderRewards>, Vec<SubnetFailureRates>)> {
        println!("Fetching node rewards for all providers from NRC from {} to {}...", start_day, end_day);

        let days: Vec<DateUtc> = start_day.iter_days().take_while(|day| day <= &end_day).map(DateUtc::from).collect();
        let responses: Vec<anyhow::Result<DailyResults>> =
            join_all(days.iter().map(|day| async move { node_rewards_client.get_rewards_daily(*day).await })).await;

        let mut providers_rewards: BTreeMap<PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>> = BTreeMap::new();
        let mut subnets_failure_rates: BTreeMap<SubnetId, Vec<(DateUtc, f64)>> = BTreeMap::new();

        for (day, response) in days.into_iter().zip(responses.into_iter()) {
            match response {
                Ok(daily_results) => {

                    for (provider_id, provider_rewards) in daily_results.provider_results {
                        providers_rewards
                            .entry(provider_id)
                            .and_modify( |results| results.push((day, provider_rewards)))
                            .or_insert_with(Vec::new);
                    }

                    for (subnet_id, failure_rate) in daily_results.subnets_failure_rate {
                        subnets_failure_rates
                            .entry(subnet_id)
                            .and_modify( |failure_rates| failure_rates.push((day, failure_rate) ))
                            .or_insert_with(Vec::new);
                    }
                }
                Err(e) => {
                    println!("Error fetching node rewards for provider: {}", e);
                }
            }
        }

        let providers_rewards = providers_rewards
            .into_iter()
            .map(|(provider_id, daily_rewards)| ProviderRewards {
                provider_id,
                nrc_total_xdr_permyriad: daily_rewards.iter().map(|(_, reward)| reward.rewards_total_xdr_permyriad.unwrap()).sum(),
                daily_rewards,
            })
            .collect();
        if let Some(filter) = provider_id {
            nrc_rewards.retain(|p| {
                let full = p.provider_id.to_string();
                let prefix = full.split('-').next().unwrap();
                full == *filter || prefix == filter
            });
        }

        let subnets_failure_rates = subnets_failure_rates
            .into_iter()
            .map(|(subnet_id, daily_failure_rates)| SubnetFailureRates {
                subnet_id,
                daily_failure_rates
            })
            .collect();

        Ok((providers_rewards, subnets_failure_rates))
    }

    fn print_rewards_summary_console(&self, provider_data: &[ProviderRewards]) -> anyhow::Result<()> {
        use tabled::settings::{Alignment, Merge, Modify, Style, Width, object::Rows};
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
                .with(Width::wrap(250).keep_words(true))
                .with(Merge::vertical());

            println!("{}", table);
        }
        Ok(())
    }
}
