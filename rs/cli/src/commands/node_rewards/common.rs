use crate::commands::node_rewards::{ProviderRewards, SubnetFailureRates};
use anyhow::Result;
use chrono::NaiveDate;
use csv::Writer;
use futures_util::future::join_all;
use ic_base_types::{PrincipalId, SubnetId};
use ic_canisters::node_rewards::NodeRewardsCanisterWrapper;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeFailureRate, DailyNodeProviderRewards, DailyResults};
use ic_node_rewards_canister_api::DateUtc;
use itertools::Itertools;
use log::info;
use std::{collections::BTreeMap, fs};
use tabled::Tabled;
pub struct ProviderRewards {
    provider_id: PrincipalId,
    nrc_total_xdr_permyriad: u64,
    daily_rewards: Vec<(DateUtc, DailyNodeProviderRewards)>,
}

pub struct SubnetFailureRates {
    subnet_id: SubnetId,
    daily_failure_rates: Vec<(DateUtc, f64)>,
}

/// Trait for generating CSV files from node rewards data
pub trait NodeRewardsCommand {
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
                            .and_modify(|results| results.push((day, provider_rewards)))
                            .or_insert_with(Vec::new);
                    }

                    for (subnet_id, failure_rate) in daily_results.subnets_failure_rate {
                        subnets_failure_rates
                            .entry(subnet_id)
                            .and_modify(|failure_rates| failure_rates.push((day, failure_rate)))
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

        let subnets_failure_rates = subnets_failure_rates
            .into_iter()
            .map(|(subnet_id, daily_failure_rates)| SubnetFailureRates {
                subnet_id,
                daily_failure_rates,
            })
            .collect();

        Ok((providers_rewards, subnets_failure_rates))
    }

    fn print_rewards_summary_console(&self, provider_data: &[ProviderRewards]) -> anyhow::Result<()> {
        use tabled::settings::{object::Rows, Alignment, Merge, Modify, Style, Width};
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

    /// Format DateUtc without the " UTC" suffix
    fn format_date_utc(date: DateUtc) -> String {
        let date_str = date.to_string();
        date_str.strip_suffix(" UTC").unwrap().to_string()
    }

    /// Generate CSV files split by provider
    async fn generate_csv_files_by_provider(
        &self,
        providers_data: &[ProviderRewards],
        output_dir: &str,
        subnets_failure_rates: &[SubnetFailureRates],
        start_day: NaiveDate,
        end_day: NaiveDate,
    ) -> anyhow::Result<()> {
        // Create rewards directory with start_day_to_end_day format
        let start_day_str = start_day.format("%Y-%m-%d").to_string();
        let end_day_str = end_day.format("%Y-%m-%d").to_string();
        let dir_name = format!("rewards_{}_to_{}", start_day_str, end_day_str);
        let rewards_dir = format!("{}/{}", output_dir, dir_name);
        fs::create_dir_all(&rewards_dir).unwrap();
        info!("Created rewards directory: {}", rewards_dir);

        // Generate CSV files for each provider separately
        for ProviderRewards { provider_id, nrc_total_xdr_permyriad: _, daily_rewards } in providers_data {
            let provider_dir = format!("{}/{}", rewards_dir, provider_id);
            fs::create_dir_all(&provider_dir)?;

            self.create_base_rewards_csv(&provider_dir, daily_rewards)?;
            self.create_base_rewards_type3_csv(&provider_dir, daily_rewards)?;
            self.create_rewards_summary_csv(&provider_dir, daily_rewards)?;
            self.create_node_metrics_csv(&provider_dir, daily_rewards)?;
        }

        // Generate subnets failure rates CSV in the rewards directory
        self.create_subnets_failure_rates_csv(&rewards_dir, subnets_failure_rates)?;

        Ok(())
    }

    /// Create base rewards CSV file
    fn create_base_rewards_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        let filename = format!("{}/base_rewards.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(["day_utc", "monthly_xdr_permyriad", "daily_xdr_permyriad", "node_reward_type", "region"])
            .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for base_reward in &rewards.base_rewards {
                wtr.write_record([
                    &day_str,
                    &base_reward.monthly_xdr_permyriad.unwrap().to_string(),
                    &base_reward.daily_xdr_permyriad.unwrap().to_string(),
                    base_reward.node_reward_type.as_ref().unwrap(),
                    base_reward.region.as_ref().unwrap(),
                ])
                    .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create base rewards type3 CSV file
    fn create_base_rewards_type3_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        let filename = format!("{}/base_rewards_type3.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record([
            "day_utc",
            "region",
            "nodes_count",
            "avg_rewards_xdr_permyriad",
            "avg_coefficient",
            "daily_xdr_permyriad",
        ])?;

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for base_reward_type3 in &rewards.base_rewards_type3 {
                wtr.write_record([
                    &day_str,
                    base_reward_type3.region.as_ref().unwrap(),
                    &base_reward_type3.nodes_count.unwrap().to_string(),
                    &base_reward_type3.avg_rewards_xdr_permyriad.unwrap().to_string(),
                    &base_reward_type3.avg_coefficient.unwrap().to_string(),
                    &base_reward_type3.daily_xdr_permyriad.unwrap().to_string(),
                ])?;
            }
        }

        wtr.flush()?;
        Ok(())
    }

    /// Create rewards summary CSV file
    fn create_rewards_summary_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        let filename = format!("{}/rewards_summary.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record([
            "day_utc",
            "base_rewards_total",
            "adjusted_rewards_total",
            "adjusted_rewards_percent",
            "rewards_total_xdr_permyriad",
            "nodes_in_registry",
            "assigned_nodes",
            "underperforming_nodes_count",
            "underperforming_nodes",
        ])
            .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);

            // Sum base and adjusted rewards across all nodes for the day
            let base_rewards_total: u64 = rewards.daily_nodes_rewards.iter().map(|n| n.base_rewards_xdr_permyriad.unwrap()).sum();

            let adjusted_rewards_total: u64 = rewards
                .daily_nodes_rewards
                .iter()
                .map(|n| n.adjusted_rewards_xdr_permyriad.unwrap())
                .sum();

            // Calculate adjusted rewards percentage
            let adjusted_rewards_percent = if base_rewards_total > 0 {
                format!("{:.2}", (adjusted_rewards_total as f64 / base_rewards_total as f64) * 100.0)
            } else {
                "N/A".to_string()
            };

            let total_rewards = rewards.rewards_total_xdr_permyriad.unwrap();
            let nodes_in_registry = rewards.daily_nodes_rewards.len();

            // Count assigned nodes
            let assigned_count = rewards
                .daily_nodes_rewards
                .iter()
                .filter(|node_result| matches!(node_result.daily_node_failure_rate, Some(DailyNodeFailureRate::SubnetMember { .. })))
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
            let underperforming_nodes = underperf_prefixes.join(", ");

            wtr.write_record([
                &day_str,
                &base_rewards_total.to_string(),
                &adjusted_rewards_total.to_string(),
                &adjusted_rewards_percent,
                &total_rewards.to_string(),
                &nodes_in_registry.to_string(),
                &assigned_count.to_string(),
                &underperforming_nodes_count.to_string(),
                &underperforming_nodes,
            ])
                .unwrap();
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create node metrics CSV files: by day, by node, and by performance_multiplier
    fn create_node_metrics_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> anyhow::Result<()> {
        // Writers for the three views
        let filename_day = format!("{}/node_metrics_by_day.csv", output_dir);
        let filename_node = format!("{}/node_metrics_by_node.csv", output_dir);
        let filename_perf = format!("{}/node_metrics_by_performance_multiplier.csv", output_dir);
        let mut wtr_day = Writer::from_path(&filename_day).unwrap();
        let mut wtr_node = Writer::from_path(&filename_node).unwrap();
        let mut wtr_perf = Writer::from_path(&filename_perf).unwrap();
        // Collector to group rows by node for the node-centric view
        let mut by_node: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
        // Collector for performance_multiplier sorted view
        let mut by_performance: Vec<(f64, DateUtc, Vec<String>)> = Vec::new();

        // Headers
        // By day: day first
        wtr_day
            .write_record([
                "day_utc",
                "node_id",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "subnet_assigned",
                "subnet_assigned_failure_rate",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_failure_rate",
                "relative_failure_rate",
                "extrapolated_failure_rate",
            ])
            .unwrap();

        // By node: node first
        wtr_node
            .write_record([
                "node_id",
                "day_utc",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "subnet_assigned",
                "subnet_assigned_failure_rate",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_failure_rate",
                "relative_failure_rate",
                "extrapolated_failure_rate",
            ])
            .unwrap();

        // By performance_multiplier: same header as by-day
        wtr_perf
            .write_record([
                "day_utc",
                "node_id",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "base_rewards_xdr_permyriad",
                "adjusted_rewards_xdr_permyriad",
                "subnet_assigned",
                "subnet_assigned_failure_rate",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_failure_rate",
                "relative_failure_rate",
                "extrapolated_failure_rate",
            ])
            .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for node_result in &rewards.daily_nodes_rewards {
                // Flatten DailyNodeFailureRate/NodeMetricsDaily
                let (
                    status_str,
                    subnet_assigned,
                    subnet_assigned_fr,
                    num_blocks_proposed,
                    num_blocks_failed,
                    original_fr,
                    relative_fr,
                    extrapolated_fr,
                ) = match &node_result.daily_node_failure_rate {
                    Some(DailyNodeFailureRate::SubnetMember { node_metrics: Some(m) }) => (
                        "Assigned".to_string(),
                        m.subnet_assigned.map(|s| s.to_string()).unwrap_or_default(),
                        m.subnet_assigned_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                        m.num_blocks_proposed.map(|n| n.to_string()).unwrap_or_default(),
                        m.num_blocks_failed.map(|n| n.to_string()).unwrap_or_default(),
                        m.original_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                        m.relative_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                        String::new(),
                    ),
                    Some(DailyNodeFailureRate::NonSubnetMember { extrapolated_failure_rate }) => (
                        "Unassigned".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        extrapolated_failure_rate.map(|f| f.to_string()).unwrap_or_default(),
                    ),
                    _ => (
                        "Unknown".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ),
                };

                // Row for by-day
                wtr_day
                    .write_record([
                        &day_str,
                        &node_result.node_id.map(|id| id.to_string()).unwrap_or_default(),
                        node_result.node_reward_type.as_ref().unwrap(),
                        node_result.region.as_ref().unwrap(),
                        node_result.dc_id.as_ref().unwrap(),
                        &status_str,
                        &node_result.performance_multiplier.unwrap().to_string(),
                        &node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                        &node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                        &subnet_assigned,
                        &subnet_assigned_fr,
                        &num_blocks_proposed,
                        &num_blocks_failed,
                        &original_fr,
                        &relative_fr,
                        &extrapolated_fr,
                    ])
                    .unwrap();

                // Collect row for by-node (omit node_id here, we'll prepend it when writing)
                by_node
                    .entry(node_result.node_id.map(|id| id.to_string()).unwrap_or_default())
                    .or_default()
                    .push(vec![
                        day_str.clone(),
                        node_result.node_reward_type.as_ref().unwrap().to_string(),
                        node_result.region.as_ref().unwrap().to_string(),
                        node_result.dc_id.as_ref().unwrap().to_string(),
                        status_str.clone(),
                        node_result.performance_multiplier.unwrap().to_string(),
                        node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                        node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                        subnet_assigned.clone(),
                        subnet_assigned_fr.clone(),
                        num_blocks_proposed.clone(),
                        num_blocks_failed.clone(),
                        original_fr.clone(),
                        relative_fr.clone(),
                        extrapolated_fr.clone(),
                    ]);

                // Collect row for by-performance_multiplier (same format as by-day)
                let performance_multiplier = node_result.performance_multiplier.unwrap();
                let perf_row = vec![
                    day_str.clone(),
                    node_result.node_id.map(|id| id.to_string()).unwrap_or_default(),
                    node_result.node_reward_type.as_ref().unwrap().to_string(),
                    node_result.region.as_ref().unwrap().to_string(),
                    node_result.dc_id.as_ref().unwrap().to_string(),
                    status_str.clone(),
                    performance_multiplier.to_string(),
                    node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                    node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                    subnet_assigned.clone(),
                    subnet_assigned_fr.clone(),
                    num_blocks_proposed.clone(),
                    num_blocks_failed.clone(),
                    original_fr.clone(),
                    relative_fr.clone(),
                    extrapolated_fr.clone(),
                ];
                by_performance.push((performance_multiplier, *day, perf_row));
            }
        }

        // Write grouped-by-node rows: each node contiguous
        for (node_id, rows) in by_node {
            for row in rows {
                let mut full = Vec::with_capacity(1 + row.len());
                full.push(node_id.clone());
                full.extend(row);
                wtr_node.write_record(full).unwrap();
            }
        }

        // Sort by performance_multiplier ascending, then by day
        by_performance.sort_by(|a, b| {
            let perf_cmp = a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal);
            if perf_cmp == std::cmp::Ordering::Equal {
                a.1.cmp(&b.1)
            } else {
                perf_cmp
            }
        });

        // Write sorted-by-performance rows
        for (_, _, row) in by_performance {
            wtr_perf.write_record(row)?;
        }

        wtr_day.flush()?;
        wtr_node.flush()?;
        wtr_perf.flush()?;
        Ok(())
    }

    /// Create subnets failure rates CSV file
    fn create_subnets_failure_rates_csv(&self, output_dir: &str, subnets_fr_data: &[SubnetFailureRates]) -> anyhow::Result<()> {
        let filename = format!("{}/subnets_failure_rates.csv", output_dir);
        let mut wtr = Writer::from_path(&filename).unwrap();

        wtr.write_record(["subnet_id", "day_utc", "failure_rate"])?;

        // Sort by subnet_id first, then by day_utc
        subnets_fr_data.iter()
            .flat_map(|subnets_fr| subnets_fr.daily_failure_rates.iter().map(|(date, fr)| (subnets_fr.subnet_id, Self::format_date_utc(*date), *fr)))
            .sorted_by(|a, b| {
                let subnet_cmp = a.1.cmp(&b.1);
                if subnet_cmp == std::cmp::Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    subnet_cmp
                }
            })
            .map(|(subnet_id, date, fr)| (subnet_id.to_string(), date, fr.to_string()))
            .for_each(|(subnet_id, date, fr)| {
                wtr.write_record([subnet_id, date, fr]).unwrap();
            });

        wtr.flush()?;
        Ok(())
    }
}
