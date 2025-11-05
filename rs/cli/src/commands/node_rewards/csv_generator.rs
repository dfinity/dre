use anyhow::Result;
use chrono::NaiveDate;
use csv::Writer;
use ic_base_types::PrincipalId;
use ic_node_rewards_canister_api::DateUtc;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeFailureRate, DailyNodeProviderRewards};
use log::info;
use std::{collections::BTreeMap, fs};

/// Trait for generating CSV files from node rewards data
pub trait CsvGenerator {
    /// Format DateUtc without the " UTC" suffix
    fn format_date_utc(date: DateUtc) -> String {
        let date_str = date.to_string();
        date_str.strip_suffix(" UTC").unwrap().to_string()
    }

    /// Generate CSV files split by provider
    async fn generate_csv_files_by_provider(
        &self,
        provider_data: &[(PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>)],
        output_dir: &str,
        subnets_fr_data: &[(DateUtc, String, f64)],
        start_day: NaiveDate,
        end_day: NaiveDate,
    ) -> Result<()> {
        // Create rewards directory with start_day_to_end_day format
        let start_day_str = start_day.format("%Y-%m-%d").to_string();
        let end_day_str = end_day.format("%Y-%m-%d").to_string();
        let dir_name = format!("rewards_{}_to_{}", start_day_str, end_day_str);
        let rewards_dir = format!("{}/{}", output_dir, dir_name);
        fs::create_dir_all(&rewards_dir).unwrap();
        info!("Created rewards directory: {}", rewards_dir);

        // Generate CSV files for each provider separately
        for (provider_id, daily_rewards) in provider_data {
            let provider_dir = format!("{}/{}", rewards_dir, provider_id);
            fs::create_dir_all(&provider_dir).unwrap();

            self.create_base_rewards_csv(&provider_dir, daily_rewards)?;
            self.create_base_rewards_type3_csv(&provider_dir, daily_rewards)?;
            self.create_rewards_summary_csv(&provider_dir, daily_rewards)?;
            self.create_node_metrics_csv(&provider_dir, daily_rewards)?;
        }

        // Generate subnets failure rates CSV in the rewards directory
        self.create_subnets_failure_rates_csv(&rewards_dir, subnets_fr_data)?;

        Ok(())
    }

    /// Create base rewards CSV file
    fn create_base_rewards_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Result<()> {
        let filename = format!("{}/base_rewards.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&["day_utc", "monthly_xdr_permyriad", "daily_xdr_permyriad", "node_reward_type", "region"])
            .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for base_reward in &rewards.base_rewards {
                wtr.write_record([
                    &day_str,
                    &base_reward.monthly_xdr_permyriad.unwrap().to_string(),
                    &base_reward.daily_xdr_permyriad.unwrap().to_string(),
                    &base_reward.node_reward_type.as_ref().unwrap(),
                    &base_reward.region.as_ref().unwrap(),
                ])
                .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create base rewards type3 CSV file
    fn create_base_rewards_type3_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Result<()> {
        let filename = format!("{}/base_rewards_type3.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&[
            "day_utc",
            "value_xdr_permyriad",
            "region",
            "nodes_count",
            "avg_rewards_xdr_permyriad",
            "avg_coefficient",
        ])
        .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = Self::format_date_utc(*day);
            for base_reward_type3 in &rewards.base_rewards_type3 {
                wtr.write_record(&[
                    &day_str,
                    &base_reward_type3.daily_xdr_permyriad.unwrap().to_string(),
                    &base_reward_type3.region.as_ref().unwrap(),
                    &base_reward_type3.nodes_count.unwrap().to_string(),
                    &base_reward_type3.avg_rewards_xdr_permyriad.unwrap().to_string(),
                    &base_reward_type3.avg_coefficient.unwrap().to_string(),
                ])
                .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create rewards summary CSV file
    fn create_rewards_summary_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Result<()> {
        let filename = format!("{}/rewards_summary.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&[
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

            wtr.write_record(&[
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

    /// Create node metrics CSV files: by day and by node
    fn create_node_metrics_csv(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Result<()> {
        // Writers for the two views
        let filename_day = format!("{}/node_metrics_by_day.csv", output_dir);
        let filename_node = format!("{}/node_metrics_by_node.csv", output_dir);
        let mut wtr_day = Writer::from_path(&filename_day).unwrap();
        let mut wtr_node = Writer::from_path(&filename_node).unwrap();
        // Collector to group rows by node for the node-centric view
        let mut by_node: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();

        // Headers
        // By day: day first
        wtr_day
            .write_record(&[
                "day_utc",
                "node_id",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "rewards_reduction",
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
            .write_record(&[
                "node_id",
                "day_utc",
                "node_reward_type",
                "region",
                "dc_id",
                "node_status",
                "performance_multiplier",
                "rewards_reduction",
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
                    .write_record(&[
                        &day_str,
                        &node_result.node_id.map(|id| id.to_string()).unwrap_or_default(),
                        &node_result.node_reward_type.as_ref().unwrap(),
                        &node_result.region.as_ref().unwrap(),
                        &node_result.dc_id.as_ref().unwrap(),
                        &status_str,
                        &node_result.performance_multiplier.unwrap().to_string(),
                        &node_result.rewards_reduction.unwrap().to_string(),
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
                    .or_insert_with(Vec::new)
                    .push(vec![
                        day_str.clone(),
                        node_result.node_reward_type.as_ref().unwrap().to_string(),
                        node_result.region.as_ref().unwrap().to_string(),
                        node_result.dc_id.as_ref().unwrap().to_string(),
                        status_str,
                        node_result.performance_multiplier.unwrap().to_string(),
                        node_result.rewards_reduction.unwrap().to_string(),
                        node_result.base_rewards_xdr_permyriad.unwrap().to_string(),
                        node_result.adjusted_rewards_xdr_permyriad.unwrap().to_string(),
                        subnet_assigned,
                        subnet_assigned_fr,
                        num_blocks_proposed,
                        num_blocks_failed,
                        original_fr,
                        relative_fr,
                        extrapolated_fr,
                    ]);
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

        wtr_day.flush().unwrap();
        wtr_node.flush().unwrap();
        Ok(())
    }

    /// Create subnets failure rates CSV file
    fn create_subnets_failure_rates_csv(&self, output_dir: &str, subnets_fr_data: &[(DateUtc, String, f64)]) -> Result<()> {
        let filename = format!("{}/subnets_failure_rates.csv", output_dir);
        let mut wtr = Writer::from_path(&filename).unwrap();

        wtr.write_record(&["subnet_id", "day_utc", "failure_rate"]).unwrap();

        // Sort by subnet_id first, then by day_utc
        let mut sorted_data: Vec<_> = subnets_fr_data.iter().collect();
        sorted_data.sort_by(|a, b| {
            let subnet_cmp = a.1.cmp(&b.1);
            if subnet_cmp == std::cmp::Ordering::Equal {
                a.0.cmp(&b.0)
            } else {
                subnet_cmp
            }
        });

        for (day, subnet_id, fr) in sorted_data {
            let day_str = Self::format_date_utc(*day);
            wtr.write_record(&[subnet_id, &day_str, &fr.to_string()]).unwrap();
        }

        wtr.flush().unwrap();
        Ok(())
    }
}
