use std::{collections::BTreeMap, fs};

use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};
use clap::Args;
use csv::Writer;
use futures_util::future::join_all;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DayUtc, NodeProviderRewardsDaily, NodeStatus};
use ic_types::PrincipalId;
use log::info;
use rewards_calculation::types::DayUtc as RewardsDayUtc;
use rust_decimal::Decimal;
use tabled::{
    settings::{object::Rows, Alignment, Margin, Modify, Padding, Style, Width},
    Table, Tabled,
};

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
    daily_rewards: Vec<NodeProviderRewardsDaily>,
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
    /// Convert DayUtc to RewardsDayUtc for display
    fn convert_day_utc(day_utc: &DayUtc) -> RewardsDayUtc {
        RewardsDayUtc::from(day_utc.value.unwrap_or(0))
    }

    /// Extract provider prefix (first part before the first '-')
    fn get_provider_prefix(provider_id: &str) -> &str {
        provider_id.split('-').next().unwrap_or(provider_id)
    }

    /// Create CSV files for node metrics - one file per node with all days
    fn create_node_metrics_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewardsDaily]) -> anyhow::Result<()> {
        // Collect all unique nodes across all days
        let mut nodes_data: BTreeMap<
            String,
            Vec<(
                RewardsDayUtc,
                String,
                String,
                String,
                String,
                Decimal,
                u64,
                u64,
                Decimal,
                Decimal,
                Decimal,
                Decimal,
                Decimal,
                Decimal,
                Decimal,
                String,
            )>,
        > = BTreeMap::new();

        for daily_reward in daily_rewards.iter().cloned() {
            let day_utc = daily_reward.day_utc.unwrap();
            let rewards_day_utc = Self::convert_day_utc(&day_utc);

            if let Some(rewards) = daily_reward.node_provider_rewards {
                for node_result in &rewards.nodes_results {
                    let node_id = node_result.node_id.as_ref().unwrap().to_string();

                    let (
                        node_status_type,
                        subnet_assigned,
                        subnet_assigned_fr_percent,
                        num_blocks_proposed,
                        num_blocks_failed,
                        original_fr_percent,
                        relative_fr_percent,
                        extrapolated_fr_percent,
                    ) = match &node_result.node_status {
                        Some(NodeStatus::Assigned { node_metrics }) => {
                            let metrics = node_metrics.as_ref().unwrap();
                            (
                                "Assigned".to_string(),
                                metrics.subnet_assigned.as_ref().map(|p| p.to_string()).unwrap_or_default(),
                                Decimal::try_from(metrics.subnet_assigned_fr_percent.as_ref().unwrap().clone()).unwrap(),
                                metrics.num_blocks_proposed.unwrap_or(0),
                                metrics.num_blocks_failed.unwrap_or(0),
                                Decimal::try_from(metrics.original_fr_percent.as_ref().unwrap().clone()).unwrap(),
                                Decimal::try_from(metrics.relative_fr_percent.as_ref().unwrap().clone()).unwrap(),
                                Decimal::ZERO,
                            )
                        }
                        Some(NodeStatus::Unassigned { extrapolated_fr_percent }) => (
                            "Unassigned".to_string(),
                            String::new(),
                            Decimal::ZERO,
                            0,
                            0,
                            Decimal::ZERO,
                            Decimal::ZERO,
                            Decimal::try_from(extrapolated_fr_percent.as_ref().unwrap().clone()).unwrap(),
                        ),
                        None => (
                            "Unknown".to_string(),
                            String::new(),
                            Decimal::ZERO,
                            0,
                            0,
                            Decimal::ZERO,
                            Decimal::ZERO,
                            Decimal::ZERO,
                        ),
                    };

                    let record = (
                        rewards_day_utc,
                        node_result.node_reward_type.as_ref().unwrap().clone(),
                        node_result.region.as_ref().unwrap().clone(),
                        node_result.dc_id.as_ref().unwrap().clone(),
                        subnet_assigned,
                        subnet_assigned_fr_percent,
                        num_blocks_proposed,
                        num_blocks_failed,
                        original_fr_percent,
                        relative_fr_percent,
                        extrapolated_fr_percent,
                        Decimal::try_from(node_result.performance_multiplier_percent.as_ref().unwrap().clone()).unwrap(),
                        Decimal::try_from(node_result.rewards_reduction_percent.as_ref().unwrap().clone()).unwrap(),
                        Decimal::try_from(node_result.base_rewards_xdr_permyriad.as_ref().unwrap().clone()).unwrap(),
                        Decimal::try_from(node_result.adjusted_rewards_xdr_permyriad.as_ref().unwrap().clone()).unwrap(),
                        node_status_type,
                    );

                    nodes_data.entry(node_id).or_default().push(record);
                }
            }
        }

        // Create one CSV file per node
        for (node_id, records) in nodes_data {
            let filename = format!("{}/node_{}.csv", output_dir, node_id);
            let mut wtr = Writer::from_path(&filename).unwrap();

            wtr.write_record(&[
                "day_utc",
                "node_reward_type",
                "region",
                "dc_id",
                "subnet_assigned",
                "subnet_assigned_fr_percent",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_fr_percent",
                "relative_fr_percent",
                "extrapolated_fr_percent",
                "performance_multiplier_percent",
                "rewards_reduction_percent",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "node_status_type",
            ])
            .unwrap();

            for (
                day_utc,
                node_reward_type,
                region,
                dc_id,
                subnet_assigned,
                subnet_assigned_fr_percent,
                num_blocks_proposed,
                num_blocks_failed,
                original_fr_percent,
                relative_fr_percent,
                extrapolated_fr_percent,
                performance_multiplier_percent,
                rewards_reduction_percent,
                base_rewards_xdr_permyriad,
                adjusted_rewards_xdr_permyriad,
                node_status_type,
            ) in records
            {
                wtr.write_record(&[
                    &day_utc.to_string(),
                    &node_reward_type,
                    &region,
                    &dc_id,
                    &subnet_assigned,
                    &subnet_assigned_fr_percent.to_string(),
                    &num_blocks_proposed.to_string(),
                    &num_blocks_failed.to_string(),
                    &original_fr_percent.to_string(),
                    &relative_fr_percent.to_string(),
                    &extrapolated_fr_percent.to_string(),
                    &performance_multiplier_percent.to_string(),
                    &rewards_reduction_percent.to_string(),
                    &base_rewards_xdr_permyriad.to_string(),
                    &adjusted_rewards_xdr_permyriad.to_string(),
                    &node_status_type,
                ])
                .unwrap();
            }
            wtr.flush().unwrap();
        }
        Ok(())
    }

    /// Create CSV for base rewards per day
    fn create_base_rewards_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewardsDaily]) -> anyhow::Result<()> {
        let filename = format!("{}/base_rewards.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&["day_utc", "monthly_xdr_permyriad", "daily_xdr_permyriad", "node_reward_type", "region"])
            .unwrap();

        for daily_reward in daily_rewards {
            let day_utc = daily_reward.day_utc.unwrap();
            let rewards_day_utc = Self::convert_day_utc(&day_utc);

            if let Some(rewards) = &daily_reward.node_provider_rewards {
                for base_reward in &rewards.base_rewards {
                    wtr.write_record(&[
                        &rewards_day_utc.to_string(),
                        &Decimal::try_from(base_reward.monthly_xdr_permyriad.as_ref().unwrap().clone())
                            .unwrap()
                            .to_string(),
                        &Decimal::try_from(base_reward.daily_xdr_permyriad.as_ref().unwrap().clone())
                            .unwrap()
                            .to_string(),
                        &base_reward.node_reward_type.as_ref().unwrap(),
                        &base_reward.region.as_ref().unwrap(),
                    ])
                    .unwrap();
                }
            }
        }
        wtr.flush().unwrap();
        Ok(())
    }

    /// Create CSV for base rewards type3 per day
    fn create_base_rewards_type3_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewardsDaily]) -> anyhow::Result<()> {
        let filename = format!("{}/base_rewards_type3.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&[
            "day_utc",
            "value_xdr_permyriad",
            "region",
            "nodes_count",
            "avg_rewards_xdr_permyriad",
            "avg_coefficient_percent",
        ])
        .unwrap();

        for daily_reward in daily_rewards {
            let day_utc = daily_reward.day_utc.unwrap();
            let rewards_day_utc = Self::convert_day_utc(&day_utc);

            if let Some(rewards) = &daily_reward.node_provider_rewards {
                for base_reward_type3 in &rewards.base_rewards_type3 {
                    wtr.write_record(&[
                        &rewards_day_utc.to_string(),
                        &Decimal::try_from(base_reward_type3.value_xdr_permyriad.as_ref().unwrap().clone())
                            .unwrap()
                            .to_string(),
                        &base_reward_type3.region.as_ref().unwrap(),
                        &base_reward_type3.nodes_count.unwrap_or(0).to_string(),
                        &Decimal::try_from(base_reward_type3.avg_rewards_xdr_permyriad.as_ref().unwrap().clone())
                            .unwrap()
                            .to_string(),
                        &Decimal::try_from(base_reward_type3.avg_coefficient_percent.as_ref().unwrap().clone())
                            .unwrap()
                            .to_string(),
                    ])
                    .unwrap();
                }
            }
        }
        wtr.flush().unwrap();
        Ok(())
    }

    /// Create CSV for rewards summary per day
    fn create_rewards_summary_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewardsDaily]) -> anyhow::Result<()> {
        let filename = format!("{}/rewards_summary.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&["day_utc", "rewards_total_xdr_permyriad", "nodes_in_registry", "underperforming_nodes"])
            .unwrap();

        for daily_reward in daily_rewards {
            let day_utc = daily_reward.day_utc.unwrap();
            let rewards_day_utc = Self::convert_day_utc(&day_utc);
            let total_rewards = Decimal::try_from(
                daily_reward
                    .node_provider_rewards
                    .as_ref()
                    .unwrap()
                    .rewards_total_xdr_permyriad
                    .as_ref()
                    .unwrap()
                    .clone(),
            )
            .unwrap();

            // Count nodes and find underperforming ones
            let mut nodes_in_registry = 0;
            let mut underperforming_nodes = Vec::new();

            if let Some(rewards) = &daily_reward.node_provider_rewards {
                nodes_in_registry = rewards.nodes_results.len();

                for node_result in &rewards.nodes_results {
                    let performance_multiplier = Decimal::try_from(node_result.performance_multiplier_percent.as_ref().unwrap().clone()).unwrap();
                    if performance_multiplier < Decimal::ONE {
                        let node_id = node_result.node_id.as_ref().unwrap().to_string();
                        // Extract short name (prefix before the first '-')
                        let short_name = node_id.split('-').next().unwrap_or(&node_id);
                        underperforming_nodes.push(short_name.to_string());
                    }
                }
            }

            let underperforming_nodes_str = underperforming_nodes.join(",");

            wtr.write_record(&[
                &rewards_day_utc.to_string(),
                &total_rewards.to_string(),
                &nodes_in_registry.to_string(),
                &underperforming_nodes_str,
            ])
            .unwrap();
        }
        wtr.flush().unwrap();
        Ok(())
    }

    /// Main function that orchestrates the data processing, CSV generation, and table display
    async fn get_latest_providers_rewards(&self, canister_agent: ic_canisters::IcAgentCanisterClient) -> anyhow::Result<()> {
        // Step 1: Query and process data
        let (provider_data, output_dir) = self.query_and_process_data(canister_agent).await.unwrap();

        // Step 2: Generate CSV files if requested
        if let Some(ref output_dir) = output_dir {
            self.generate_csv_files(&provider_data, output_dir).await.unwrap();
        }

        // Step 3: Display comparison table
        self.display_comparison_table(&provider_data).await.unwrap();

        Ok(())
    }

    /// Query governance and node rewards data and process it
    async fn query_and_process_data(
        &self,
        canister_agent: ic_canisters::IcAgentCanisterClient,
    ) -> anyhow::Result<(Vec<ProviderData>, Option<String>)> {
        let node_rewards_client: NodeRewardsCanisterWrapper = canister_agent.clone().into();
        let node_rewards_client_ref = &node_rewards_client;
        let governance_client: GovernanceCanisterWrapper = canister_agent.into();

        // Get governance rewards data
        let filter = ic_nns_governance_api::DateRangeFilter {
            start_timestamp_seconds: Some(1747094400), // 13 May 2025
            end_timestamp_seconds: None,
        };
        let mut rewards = governance_client.list_node_provider_rewards(Some(filter)).await.unwrap().into_iter();

        let last = rewards.next().ok_or(anyhow::anyhow!("No rewards found")).unwrap();
        let xdr_permyriad_per_icp: Decimal =
            Decimal::try_from(last.xdr_conversion_rate.as_ref().unwrap().xdr_permyriad_per_icp.as_ref().unwrap().clone()).unwrap();
        let rewards_per_provider_icp: BTreeMap<PrincipalId, Decimal> = last
            .rewards
            .iter()
            .map(|rewards| {
                let principal_id = rewards.node_provider.as_ref().unwrap().id.unwrap();
                let icp_total = Decimal::from(rewards.amount_e8s) / Decimal::from(100000000);
                (principal_id, icp_total)
            })
            .collect();

        let start_ts = rewards.next().ok_or(anyhow::anyhow!("No rewards found")).unwrap().timestamp;
        let start_day = rewards_calculation::types::DayUtc::from_secs(start_ts);
        let end_day = rewards_calculation::types::DayUtc::from_secs(last.timestamp).previous_day();

        // Create output directory if CSV generation is requested
        let output_dir = if let Some(csv_path) = &self.csv_output_path {
            let dir = format!("{}/{}_to_{}_rewards", csv_path, start_day, end_day);
            fs::create_dir_all(&dir).unwrap();
            info!("Created output directory: {}", dir);
            Some(dir)
        } else {
            None
        };

        // Fetch node rewards for all providers from NRC concurrently
        let responses = join_all(last.node_providers.into_iter().map(|provider| async move {
            let provider_id = provider.id.unwrap();
            let daily_rewards = node_rewards_client_ref.get_provider_rewards_daily(provider_id, start_day, end_day).await;

            match daily_rewards {
                Ok(rewards) => Ok((provider_id, rewards)),
                Err(e) => Err((provider_id, e)),
            }
        }))
        .await;

        // Process results and calculate comparisons
        let mut provider_data = Vec::new();
        for response in responses {
            match response {
                Ok((provider_id, daily_rewards)) => {
                    let grand_total_xdr_permyriad_nrc: Decimal = daily_rewards
                        .iter()
                        .map(|reward| {
                            let decimal_pb = reward
                                .node_provider_rewards
                                .as_ref()
                                .unwrap()
                                .rewards_total_xdr_permyriad
                                .as_ref()
                                .unwrap();
                            Decimal::try_from(decimal_pb.clone()).unwrap()
                        })
                        .sum();
                    let grand_total_icp_nrc = grand_total_xdr_permyriad_nrc / xdr_permyriad_per_icp;
                    let grand_total_icp_governance = *rewards_per_provider_icp.get(&provider_id).unwrap();
                    let grand_total_xdr_permyriad_governance = grand_total_icp_governance * xdr_permyriad_per_icp;

                    // Calculate difference as NRC - Governance
                    let difference = grand_total_icp_nrc - grand_total_icp_governance;

                    // Collect underperforming nodes for this provider
                    let underperforming_nodes = self.collect_underperforming_nodes(&daily_rewards);

                    provider_data.push(ProviderData {
                        provider_id,
                        nrc_icp: grand_total_icp_nrc,
                        governance_icp: grand_total_icp_governance,
                        nrc_xdr_permyriad: grand_total_xdr_permyriad_nrc,
                        governance_xdr_permyriad: grand_total_xdr_permyriad_governance,
                        difference,
                        underperforming_nodes,
                        daily_rewards,
                    });
                }
                Err((provider_id, e)) => {
                    let grand_total_icp_governance = *rewards_per_provider_icp.get(&provider_id).unwrap();
                    println!(
                        "Failed to get provider rewards: {:?} governance rewards: {}",
                        e, grand_total_icp_governance
                    );
                }
            }
        }

        // Sort by percentage difference (descending)
        provider_data.sort_by(|a, b| {
            let a_base = if a.difference >= Decimal::ZERO { a.governance_icp } else { a.nrc_icp };
            let b_base = if b.difference >= Decimal::ZERO { b.governance_icp } else { b.nrc_icp };
            let a_percent = if a_base > Decimal::ZERO {
                a.difference / a_base * Decimal::ONE_HUNDRED
            } else {
                Decimal::ZERO
            };
            let b_percent = if b_base > Decimal::ZERO {
                b.difference / b_base * Decimal::ONE_HUNDRED
            } else {
                Decimal::ZERO
            };
            b_percent.partial_cmp(&a_percent).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok((provider_data, output_dir))
    }

    /// Collect underperforming nodes for a provider
    fn collect_underperforming_nodes(&self, daily_rewards: &[NodeProviderRewardsDaily]) -> Vec<String> {
        let mut underperforming_nodes = Vec::new();
        for daily_reward in daily_rewards {
            if let Some(node_provider_rewards) = &daily_reward.node_provider_rewards {
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
        // Remove duplicates and sort
        underperforming_nodes.sort();
        underperforming_nodes.dedup();
        underperforming_nodes
    }

    /// Generate CSV files for all providers
    async fn generate_csv_files(&self, provider_data: &[ProviderData], output_dir: &str) -> anyhow::Result<()> {
        for provider in provider_data {
            let provider_output_dir = format!("{}/{}", output_dir, provider.provider_id);
            fs::create_dir_all(&provider_output_dir).unwrap();

            self.create_node_metrics_csv(&provider_output_dir, &provider.daily_rewards).unwrap();
            self.create_base_rewards_csv(&provider_output_dir, &provider.daily_rewards).unwrap();
            self.create_base_rewards_type3_csv(&provider_output_dir, &provider.daily_rewards).unwrap();
            self.create_rewards_summary_csv(&provider_output_dir, &provider.daily_rewards).unwrap();

            println!(
                "CSV files created for provider {} in directory: {}",
                provider.provider_id, provider_output_dir
            );
        }
        Ok(())
    }

    /// Display the comparison table
    async fn display_comparison_table(&self, provider_data: &[ProviderData]) -> anyhow::Result<()> {
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

            let (nrc_display, governance_display, difference_display) = if self.xdr_permyriad {
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
        let unit_label = if self.xdr_permyriad { "XDRPermyriad" } else { "ICP" };
        println!("\n=== NODE REWARDS COMPARISON: NRC vs GOVERNANCE ===");
        println!("Unit: {} | Sorted by decreasing percentage difference", unit_label);
        println!("\nLegend:");
        println!("• NRC: Node Rewards Canister rewards");
        println!("• Governance: Governance rewards from NNS");
        println!("• Difference: NRC - Governance");
        println!("• % Diff: (Difference / Base Value) × 100%");
        println!("  - Base Value = Governance when Difference ≥ 0");
        println!("  - Base Value = NRC when Difference < 0");
        println!();

        let mut table = Table::new(&table_data);
        table
            .with(Style::extended())
            .with(Padding::new(1, 1, 0, 0))
            .with(Margin::new(0, 0, 0, 0))
            .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
            .with(Modify::new(Rows::new(0..1)).with(Alignment::center()))
            .with(Width::wrap(120).keep_words(true));

        println!("{}", table);

        println!("\n=== SUMMARY ===");
        println!("Successfully processed {} providers", provider_data.len());
        if self.csv_output_path.is_some() {
            println!("CSV files generated");
        } else {
            println!("CSV generation skipped (no --csv-output-path provided)");
        }

        Ok(())
    }
}

impl ExecutableCommand for NodeRewards {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Signer
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let (_, canister_agent) = ctx.create_ic_agent_canister_client().await.unwrap();
        info!("Started action...");

        self.get_latest_providers_rewards(canister_agent).await.unwrap();

        //println!("{}", serde_json::to_string_pretty(&metrics_by_subnet).unwrap());

        Ok(())
    }
}
