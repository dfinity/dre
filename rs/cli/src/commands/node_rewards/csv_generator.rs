use anyhow::Result;
use csv::Writer;
use ic_base_types::PrincipalId;
use log::info;
use rewards_calculation::performance_based_algorithm::results::NodeProviderRewards;
use rewards_calculation::types::DayUtc;
use rust_decimal::Decimal;
use std::{collections::BTreeMap, fs};

/// Trait for generating CSV files from node rewards data
pub trait CsvGenerator {
    /// Generate CSV files split by provider
    async fn generate_csv_files_by_provider(
        &self,
        provider_data: &[(PrincipalId, Vec<(DayUtc, NodeProviderRewards)>)],
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
    fn create_rewards_directory(&self, output_dir: &str, daily_rewards: &[(DayUtc, NodeProviderRewards)]) -> Result<String> {
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
    fn get_date_range(&self, daily_rewards: &[(DayUtc, NodeProviderRewards)]) -> (String, String) {
        if daily_rewards.is_empty() {
            return ("unknown".to_string(), "unknown".to_string());
        }

        let mut days: Vec<DayUtc> = daily_rewards.iter().map(|(day, _)| *day).collect();
        days.sort();

        let start_day = days.first().unwrap().to_string();
        let end_day = days.last().unwrap().to_string();

        (start_day, end_day)
    }

    /// Create base rewards CSV file
    fn create_base_rewards_csv(&self, output_dir: &str, daily_rewards: &[(DayUtc, NodeProviderRewards)]) -> Result<()> {
        let filename = format!("{}/base_rewards.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&["day_utc", "monthly_xdr_permyriad", "daily_xdr_permyriad", "node_reward_type", "region"])
            .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = day.to_string();
            for base_reward in &rewards.base_rewards {
                wtr.write_record([
                    &day_str,
                    &base_reward.monthly.trunc().to_string(),
                    &base_reward.daily.trunc().to_string(),
                    &base_reward.node_reward_type.to_string(),
                    &base_reward.region,
                ])
                .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create base rewards type3 CSV file
    fn create_base_rewards_type3_csv(&self, output_dir: &str, daily_rewards: &[(DayUtc, NodeProviderRewards)]) -> Result<()> {
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
            let day_str = day.to_string();
            for base_reward_type3 in &rewards.base_rewards_type3 {
                wtr.write_record(&[
                    &day_str,
                    &base_reward_type3.value.trunc().to_string(),
                    &base_reward_type3.region,
                    &base_reward_type3.nodes_count.to_string(),
                    &base_reward_type3.avg_rewards.trunc().to_string(),
                    &base_reward_type3.avg_coefficient.trunc().to_string(),
                ])
                .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create rewards summary CSV file
    fn create_rewards_summary_csv(&self, output_dir: &str, daily_rewards: &[(DayUtc, NodeProviderRewards)]) -> Result<()> {
        let filename = format!("{}/rewards_summary.csv", output_dir);

        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&[
            "day_utc",
            "rewards_total_xdr_permyriad",
            "nodes_in_registry",
            "assigned_nodes",
            "underperforming_nodes_count",
            "underperforming_nodes",
        ])
        .unwrap();

        for (day, rewards) in daily_rewards {
            let day_str = day.to_string();
            let total_rewards = rewards.rewards_total;
            let nodes_in_registry = rewards.nodes_results.len();

            // Count assigned nodes
            let assigned_count = rewards
                .nodes_results
                .iter()
                .filter(|node_result| {
                    matches!(
                        node_result.node_status,
                        rewards_calculation::performance_based_algorithm::results::NodeStatus::Assigned { .. }
                    )
                })
                .count();

            let mut underperf_prefixes: Vec<String> = rewards
                .nodes_results
                .iter()
                .filter(|node_result| node_result.performance_multiplier < Decimal::from(1))
                .map(|node_result| {
                    let node_id_str = node_result.node_id.to_string();
                    node_id_str.split('-').next().unwrap_or(&node_id_str).to_string()
                })
                .collect();
            underperf_prefixes.sort();
            underperf_prefixes.dedup();
            let underperforming_nodes_count = underperf_prefixes.len();
            let underperforming_nodes = underperf_prefixes.join(", ");

            wtr.write_record(&[
                &day_str,
                &total_rewards.trunc().to_string(),
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
    fn create_node_metrics_csv(&self, output_dir: &str, daily_rewards: &[(DayUtc, NodeProviderRewards)]) -> Result<()> {
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
            let day_str = day.to_string();
            for node_result in &rewards.nodes_results {
                // Flatten NodeStatus/NodeMetricsDaily
                let (
                    status_str,
                    subnet_assigned,
                    subnet_assigned_fr,
                    num_blocks_proposed,
                    num_blocks_failed,
                    original_fr,
                    relative_fr,
                    extrapolated_fr,
                ) = match &node_result.node_status {
                    rewards_calculation::performance_based_algorithm::results::NodeStatus::Assigned { node_metrics } => {
                        let m = node_metrics;
                        (
                            "Assigned".to_string(),
                            m.subnet_assigned.to_string(),
                            m.subnet_assigned_fr.to_string(),
                            m.num_blocks_proposed.to_string(),
                            m.num_blocks_failed.to_string(),
                            m.original_fr.to_string(),
                            m.relative_fr.to_string(),
                            String::new(),
                        )
                    }
                    rewards_calculation::performance_based_algorithm::results::NodeStatus::Unassigned { extrapolated_fr } => (
                        "Unassigned".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        extrapolated_fr.to_string(),
                    ),
                };

                // Row for by-day
                wtr_day
                    .write_record(&[
                        &day_str,
                        &node_result.node_id.to_string(),
                        &node_result.node_reward_type.to_string(),
                        &node_result.region,
                        &node_result.dc_id,
                        &status_str,
                        &node_result.performance_multiplier.trunc().to_string(),
                        &node_result.rewards_reduction.trunc().to_string(),
                        &node_result.base_rewards.trunc().to_string(),
                        &node_result.adjusted_rewards.trunc().to_string(),
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
                by_node.entry(node_result.node_id.to_string()).or_insert_with(Vec::new).push(vec![
                    day_str.clone(),
                    node_result.node_reward_type.to_string(),
                    node_result.region.clone(),
                    node_result.dc_id.clone(),
                    status_str,
                    node_result.performance_multiplier.trunc().to_string(),
                    node_result.rewards_reduction.trunc().to_string(),
                    node_result.base_rewards.trunc().to_string(),
                    node_result.adjusted_rewards.trunc().to_string(),
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
