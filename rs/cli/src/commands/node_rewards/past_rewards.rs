use crate::commands::node_rewards::common::{NodeRewardsCommand, ProviderRewards};
use chrono::{DateTime, Datelike};
use ic_base_types::PrincipalId;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use std::collections::BTreeMap;
use tabled::settings::object::{Columns, Rows};
use tabled::settings::{Alignment, Merge, Modify, Style, Width};
use tabled::{Table, Tabled};

#[derive(Tabled)]
pub struct ProviderComparison {
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
    #[tabled(rename = "Underperf Nodes")]
    underperforming_nodes: String,
}

pub struct PastRewardsCommand;

impl NodeRewardsCommand for PastRewardsCommand {}

impl PastRewardsCommand {
    pub async fn run(
        canister_agent: ic_canisters::IcAgentCanisterClient,
        month: &str,
        csv_detailed_output_path: Option<&str>,
        provider_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        let gov_rewards_list = governance_client.list_node_provider_rewards(None).await?;
        let target = chrono::NaiveDate::parse_from_str(&(month.to_string() + "-01"), "%Y-%m-%d")?;
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
        let i = idx_in_month.ok_or_else(|| anyhow::anyhow!("No governance snapshot found for {}", month))?;
        let last = &gov_rewards_list[i];
        let xdr_permyriad_per_icp = last
            .xdr_conversion_rate
            .as_ref()
            .and_then(|x| x.xdr_permyriad_per_icp)
            .ok_or_else(|| anyhow::anyhow!("Missing XDR conversion rate"))?;
        let gov_providers_rewards: BTreeMap<PrincipalId, u64> = last
            .rewards
            .iter()
            .map(|r| {
                let icp_amount = r.amount_e8s as f64 / 100_000_000f64;
                let xdr_permyriad_amount = icp_amount * xdr_permyriad_per_icp as f64;
                (r.node_provider.as_ref().and_then(|np| np.id).unwrap(), xdr_permyriad_amount as u64)
            })
            .collect();

        let prev = gov_rewards_list
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("Previous governance snapshot not found for {}", month))?;

        let start_date = DateTime::from_timestamp(prev.timestamp as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid previous timestamp"))?
            .date_naive();
        let end_date = DateTime::from_timestamp(last.timestamp as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid last timestamp"))?
            .date_naive()
            .pred_opt()
            .ok_or_else(|| anyhow::anyhow!("Cannot get previous day"))?;

        let command = PastRewardsCommand;
        let (mut nrc_providers_rewards, subnets_failure_rates) = command.fetch_nrc_rewards(&node_rewards_client, start_date, end_date).await?;

        if let Some(provider_filter) = provider_id {
            nrc_providers_rewards.retain(|p| {
                let provider_id_str = p.provider_id.to_string();
                let prefix = PastRewardsCommand::get_provider_prefix(&provider_id_str);
                provider_id_str == provider_filter || prefix == provider_filter
            });
        }

        if let Some(output_path) = csv_detailed_output_path {
            command
                .generate_csv_files_by_provider(&nrc_providers_rewards, output_path, &subnets_failure_rates, start_date, end_date)
                .await?;
        } else {
            command.print_rewards_summary_console(&nrc_providers_rewards)?;
        }

        command.display_comparison_table(&nrc_providers_rewards, &gov_providers_rewards).await?;
        Ok(())
    }

    /// Display the comparison table
    async fn display_comparison_table(
        &self,
        provider_data: &[ProviderRewards],
        gov_providers_rewards: &BTreeMap<PrincipalId, u64>,
    ) -> anyhow::Result<()> {
        // Create table data with percentage values for sorting
        let mut table_data_with_pct: Vec<(f64, ProviderComparison)> = Vec::new();
        for provider in provider_data {
            let provider_id_str = provider.provider_id.to_string();
            let provider_prefix = PastRewardsCommand::get_provider_prefix(&provider_id_str);

            let gov_reward = gov_providers_rewards.get(&provider.provider_id).copied().unwrap_or(0);
            let nrc_total = provider.nrc_total_xdr_permyriad;

            // Calculate difference as signed to handle negative values
            let difference = nrc_total as i64 - gov_reward as i64;

            // Calculate percentage difference always relative to NRC, always display in XDRPermyriad
            let percent_diff = if nrc_total > 0 {
                difference as f64 / nrc_total as f64 * 100.0
            } else {
                0.0
            };

            // Always display in XDRPermyriad
            let (nrc_display, governance_display, difference_display) = (nrc_total.to_string(), gov_reward.to_string(), difference.to_string());

            // Collect all underperforming nodes across all days for this provider
            let underperf_prefixes = self.collect_underperforming_nodes(&provider.daily_rewards);
            let underperforming_nodes_display = if underperf_prefixes.is_empty() {
                "None".to_string()
            } else {
                underperf_prefixes.join(", ")
            };

            table_data_with_pct.push((
                percent_diff,
                ProviderComparison {
                    provider: provider_prefix.to_string(),
                    nrc_rewards: nrc_display,
                    governance_rewards: governance_display,
                    difference: difference_display,
                    percent_diff: format!("{:.2}%", percent_diff),
                    underperforming_nodes: underperforming_nodes_display,
                },
            ));
        }

        // Sort by percentage difference (descending) - use absolute value for sorting
        table_data_with_pct.sort_by(|a, b| {
            let a_abs = a.0.abs();
            let b_abs = b.0.abs();
            b_abs.partial_cmp(&a_abs).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Extract just the ProviderComparison entries
        let table_data: Vec<ProviderComparison> = table_data_with_pct.into_iter().map(|(_, entry)| entry).collect();

        // Create and display table
        println!("\n=== NODE REWARDS COMPARISON: NRC vs GOVERNANCE ===");
        println!("Unit: XDRPermyriad | Sorted by decreasing percentage difference");
        println!("\nLegend:");
        println!("• NRC: Node Rewards Canister rewards (XDRPermyriad)");
        println!("• Governance: Governance rewards from NNS (XDRPermyriad)");
        println!("• Difference: NRC - Governance (XDRPermyriad)");
        println!("• % Diff: (Difference / Base Value) × 100%");
        println!("• Underperf Nodes: Comma-separated list of underperforming node IDs (prefixes)");
        println!();

        let mut table = Table::new(table_data);
        table
            .with(Style::modern())
            .with(Modify::new(Rows::new(0..1)).with(Alignment::center()))
            .with(Modify::new(Columns::new(5..6)).with(Width::truncate(60).suffix("...")))
            .with(Width::wrap(200).keep_words(true))
            .with(Merge::vertical());

        println!("{}", table);

        println!("\n=== SUMMARY ===");
        println!("Successfully processed {} providers", provider_data.len());

        Ok(())
    }
}
