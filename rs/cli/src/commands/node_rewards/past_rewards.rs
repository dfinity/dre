use super::{fetch_and_aggregate, DateUtc, NodeRewards, ProviderComparison, ProviderRewards};
use crate::commands::node_rewards::common::NodeRewardsCommand;
use chrono::{DateTime, Datelike};
use ic_base_types::PrincipalId;
use std::collections::BTreeMap;
use tabled::settings::object::Rows;
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
    #[tabled(rename = "Underperforming Nodes")]
    underperforming_nodes: String,
}


pub struct PastRewardsCommand;

impl NodeRewardsCommand for PastRewardsCommand {}
impl PastRewardsCommand {
    pub async fn run(
        canister_agent: ic_canisters::IcAgentCanisterClient,
        cmd: &NodeRewards,
        month: &str,
    ) -> anyhow::Result<(chrono::NaiveDate, chrono::NaiveDate, Vec<ProviderRewards>, Vec<(DateUtc, String, f64)>)> {
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
        let xdr_permyriad_per_icp = last.xdr_conversion_rate.unwrap().xdr_permyriad_per_icp.unwrap();
        let gov_providers_rewards: BTreeMap<PrincipalId, u64> = last
            .rewards
            .into_iter()
            .map(|r| {
                let icp_amount = r.amount_e8s as f64 / 100_000_000f64;
                let xdr_permyriad_amount = icp_amount * xdr_permyriad_per_icp as f64;

                (r.node_provider.unwrap().id.unwrap(), xdr_permyriad_amount as u64)
            })
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

        Ok((start_day, end_day, provider_data, subnets_fr))
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
