use anyhow::Result;
use csv::Writer;
use ic_base_types::PrincipalId;
use ic_node_rewards_canister_api::provider_rewards_calculation::{DailyNodeFailureRate, DailyNodeProviderRewards};
use ic_node_rewards_canister_api::DateUtc;
use log::info;
use std::{collections::BTreeMap, fs};

/// Trait for generating CSV files from node rewards data
pub trait CsvGenerator {
    /// Format DateUtc without the " UTC" suffix
    fn format_date_utc(date: DateUtc) -> String {
        let date_str = date.to_string();
        date_str.strip_suffix(" UTC").unwrap_or(&date_str).to_string()
    }

    /// Generate CSV files split by provider
    async fn generate_csv_files_by_provider(
        &self,
        provider_data: &[(PrincipalId, Vec<(DateUtc, DailyNodeProviderRewards)>)],
        output_dir: &str,
    ) -> Result<()> {
        // Create rewards directory with start_day_to_end_day format
        let rewards_dir = self.create_rewards_directory(
            output_dir,
            &provider_data.iter().flat_map(|(_, rewards)| rewards.clone()).collect::<Vec<_>>(),
        )?;
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

        Ok(())
    }

    /// Create rewards directory with start_day_to_end_day format
    fn create_rewards_directory(&self, output_dir: &str, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> Result<String> {
        // Get the date range from daily rewards
        let (start_day, end_day) = self.get_date_range(daily_rewards);

        // Create directory name in format: rewards_start_day_to_end_day
        let dir_name = format!("rewards_{}_to_{}", start_day, end_day);
        let rewards_dir = format!("{}/{}", output_dir, dir_name);

        // Create the directory
        fs::create_dir_all(&rewards_dir).unwrap();

        Ok(rewards_dir)
    }

    /// Get the date range from daily rewards
    fn get_date_range(&self, daily_rewards: &[(DateUtc, DailyNodeProviderRewards)]) -> (String, String) {
        if daily_rewards.is_empty() {
            return ("unknown".to_string(), "unknown".to_string());
        }

        let mut days: Vec<DateUtc> = daily_rewards.iter().map(|(day, _)| *day).collect();
        days.sort();

        let start_day = Self::format_date_utc(days[0]);
        let end_day = Self::format_date_utc(days[days.len() - 1]);

        (start_day, end_day)
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
                    &base_reward.monthly_xdr_permyriad.unwrap_or(0).to_string(),
                    &base_reward.daily_xdr_permyriad.unwrap_or(0).to_string(),
                    &base_reward.node_reward_type.as_ref().unwrap_or(&String::new()),
                    &base_reward.region.as_ref().unwrap_or(&String::new()),
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
                    &base_reward_type3.daily_xdr_permyriad.unwrap_or(0).to_string(),
                    &base_reward_type3.region.as_ref().unwrap_or(&String::new()),
                    &base_reward_type3.nodes_count.unwrap_or(0).to_string(),
                    &base_reward_type3.avg_rewards_xdr_permyriad.unwrap_or(0).to_string(),
                    &base_reward_type3.avg_coefficient_percent.unwrap_or(0.0).to_string(),
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
            let base_rewards_total: u64 = rewards
                .daily_nodes_rewards
                .iter()
                .map(|n| n.base_rewards_xdr_permyriad.unwrap_or(0))
                .sum();

            let adjusted_rewards_total: u64 = rewards
                .daily_nodes_rewards
                .iter()
                .map(|n| n.adjusted_rewards_xdr_permyriad.unwrap_or(0))
                .sum();

            // Calculate adjusted rewards percentage
            let adjusted_rewards_percent = if base_rewards_total > 0 {
                format!("{:.2}", (adjusted_rewards_total as f64 / base_rewards_total as f64) * 100.0)
            } else {
                "N/A".to_string()
            };

            let total_rewards = rewards.rewards_total_xdr_permyriad.unwrap_or(0);
            let nodes_in_registry = rewards.daily_nodes_rewards.len();

            // Count assigned nodes
            let assigned_count = rewards
                .daily_nodes_rewards
                .iter()
                .filter(|node_result| matches!(node_result.daily_node_fr, Some(DailyNodeFailureRate::SubnetMember { .. })))
                .count();

            let mut underperf_prefixes: Vec<String> = rewards
                .daily_nodes_rewards
                .iter()
                .filter(|node_result| node_result.performance_multiplier_percent.unwrap_or(1.0) < 1.0)
                .map(|node_result| {
                    let node_id_str = node_result.node_id.unwrap().to_string();
                    node_id_str.split('-').next().unwrap_or(&node_id_str).to_string()
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
                "subnet_assigned_fr",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_fr",
                "relative_fr",
                "extrapolated_fr",
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
                "subnet_assigned_fr",
                "num_blocks_proposed",
                "num_blocks_failed",
                "original_fr",
                "relative_fr",
                "extrapolated_fr",
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
                ) = match &node_result.daily_node_fr {
                    Some(DailyNodeFailureRate::SubnetMember { node_metrics: Some(m) }) => (
                        "Assigned".to_string(),
                        m.subnet_assigned.map(|s| s.to_string()).unwrap_or_default(),
                        m.subnet_assigned_fr_percent.map(|f| f.to_string()).unwrap_or_default(),
                        m.num_blocks_proposed.map(|n| n.to_string()).unwrap_or_default(),
                        m.num_blocks_failed.map(|n| n.to_string()).unwrap_or_default(),
                        m.original_fr_percent.map(|f| f.to_string()).unwrap_or_default(),
                        m.relative_fr_percent.map(|f| f.to_string()).unwrap_or_default(),
                        String::new(),
                    ),
                    Some(DailyNodeFailureRate::NonSubnetMember { extrapolated_fr_percent }) => (
                        "Unassigned".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        extrapolated_fr_percent.map(|f| f.to_string()).unwrap_or_default(),
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
                        &node_result.node_reward_type.as_ref().unwrap_or(&String::new()),
                        &node_result.region.as_ref().unwrap_or(&String::new()),
                        &node_result.dc_id.as_ref().unwrap_or(&String::new()),
                        &status_str,
                        &node_result.performance_multiplier_percent.unwrap_or(0.0).to_string(),
                        &node_result.rewards_reduction_percent.unwrap_or(0.0).to_string(),
                        &node_result.base_rewards_xdr_permyriad.unwrap_or(0).to_string(),
                        &node_result.adjusted_rewards_xdr_permyriad.unwrap_or(0).to_string(),
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
                        node_result.node_reward_type.as_ref().unwrap_or(&String::new()).to_string(),
                        node_result.region.as_ref().unwrap_or(&String::new()).to_string(),
                        node_result.dc_id.as_ref().unwrap_or(&String::new()).to_string(),
                        status_str,
                        node_result.performance_multiplier_percent.unwrap_or(0.0).to_string(),
                        node_result.rewards_reduction_percent.unwrap_or(0.0).to_string(),
                        node_result.base_rewards_xdr_permyriad.unwrap_or(0).to_string(),
                        node_result.adjusted_rewards_xdr_permyriad.unwrap_or(0).to_string(),
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
}
