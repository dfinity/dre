use anyhow::Result;
use csv::Writer;
use log::info;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::{collections::BTreeMap, fs};
use ic_base_types::PrincipalId;
use rewards_calculation::performance_based_algorithm::results::NodeProviderRewards;
use rewards_calculation::types::DayUtc;

/// Trait for generating CSV files from node rewards data
pub trait CsvGenerator {
    /// Generate CSV files split by provider
    async fn generate_csv_files_by_provider(&self, provider_data: &[(PrincipalId, Vec<NodeProviderRewards>)], output_dir: &str) -> Result<()> {
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
            info!("Created provider directory: {}", provider_dir);

            self.create_base_rewards_csv(&provider_dir, daily_rewards)?;
            self.create_base_rewards_type3_csv(&provider_dir, daily_rewards)?;
            self.create_rewards_summary_csv(&provider_dir, daily_rewards)?;
            self.create_node_metrics_csv(&provider_dir, daily_rewards)?;
        }

        Ok(())
    }

    /// Create rewards directory with start_day_to_end_day format
    fn create_rewards_directory(&self, output_dir: &str, daily_rewards: &[NodeProviderRewards]) -> Result<String> {
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
    fn get_date_range(&self, daily_rewards: &[NodeProviderRewards]) -> (String, String) {
        if daily_rewards.is_empty() {
            return ("unknown".to_string(), "unknown".to_string());
        }

        let mut days: Vec<DayUtc> = daily_rewards
            .iter()
            .filter_map(|reward| reward.day_utc.as_ref().map(|day| RCDayUtc::from_nanos(day.value.unwrap())))
            .collect();

        days.sort();

        let start_day = days.first().unwrap().to_string();
        let end_day = days.last().unwrap().to_string();

        (start_day, end_day)
    }

    /// Convert daily rewards to CSV format
    async fn convert_daily_rewards_to_csv(&self, daily_rewards: &[NodeProviderRewards], provider_id: PrincipalId) -> Result<String> {
        let mut csv_data = "Day,Provider ID,XDRPermyriad Rewards,Node Count,Avg Rewards XDRPermyriad,Avg Coefficient %\n".to_string();

        // CSV rows
        for daily_reward in daily_rewards {
            let day = daily_reward.day_utc.unwrap().value.unwrap_or(0);

            if let Some(node_provider_rewards) = &daily_reward.node_provider_rewards {
                // Use the provided provider ID
                let provider_id_str = provider_id.to_string();

                let rewards_value = node_provider_rewards.rewards_total_xdr_permyriad.as_ref().unwrap().clone();

                let rewards_decimal = Decimal::try_from(rewards_value).unwrap();
                let node_count = node_provider_rewards.nodes_results.len();

                // Calculate average rewards and coefficient from the data
                let avg_rewards = if node_count > 0 {
                    rewards_decimal / Decimal::from(node_count)
                } else {
                    Decimal::ZERO
                };

                // Calculate average coefficient from node results
                let avg_coefficient = if node_count > 0 {
                    let total_coefficient: Decimal = node_provider_rewards
                        .nodes_results
                        .iter()
                        .map(|node_result| Decimal::try_from(node_result.performance_multiplier_percent.as_ref().unwrap().clone()).unwrap())
                        .sum();
                    let avg_coefficient_decimal = total_coefficient / Decimal::from(node_count);
                    avg_coefficient_decimal.to_f64().unwrap_or(0.0)
                } else {
                    0.0
                };

                csv_data.push_str(&format!(
                    "{},{},{:.0},{},{:.0},{:.2}\n",
                    day, provider_id_str, rewards_decimal, node_count, avg_rewards, avg_coefficient
                ));
            }
        }

        Ok(csv_data)
    }

    /// Convert DayUtc to RewardsDayUtc for display
    fn convert_day_utc(day_utc: &DayUtc) -> RewardsDayUtc {
        RewardsDayUtc::from(day_utc.value.unwrap_or(0))
    }

    /// Create base rewards CSV file
    fn create_base_rewards_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewards]) -> Result<()> {
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
                        &base_reward.node_reward_type.as_ref().unwrap_or(&"unknown".to_string()),
                        &base_reward.region.as_ref().unwrap_or(&"unknown".to_string()),
                    ])
                    .unwrap();
                }
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create base rewards type3 CSV file
    fn create_base_rewards_type3_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewards]) -> Result<()> {
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
                        &base_reward_type3.region.as_ref().unwrap_or(&"unknown".to_string()),
                        &base_reward_type3.nodes_count.unwrap_or(0).to_string(),
                        &Decimal::try_from(base_reward_type3.avg_rewards_xdr_permyriad.as_ref().unwrap().clone())
                            .unwrap()
                            .to_string(),
                        &base_reward_type3
                            .avg_coefficient_percent
                            .as_ref()
                            .and_then(|d| d.human_readable.as_ref())
                            .unwrap_or(&"0".to_string()),
                    ])
                    .unwrap();
                }
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create rewards summary CSV file
    fn create_rewards_summary_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewards]) -> Result<()> {
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
            let nodes_in_registry = daily_reward.node_provider_rewards.as_ref().unwrap().nodes_results.len();
            let underperforming_nodes = daily_reward
                .node_provider_rewards
                .as_ref()
                .unwrap()
                .nodes_results
                .iter()
                .filter(|node_result| {
                    let multiplier = Decimal::try_from(node_result.performance_multiplier_percent.as_ref().unwrap().clone()).unwrap();
                    multiplier < Decimal::from(1)
                })
                .count();

            wtr.write_record(&[
                &rewards_day_utc.to_string(),
                &total_rewards.to_string(),
                &nodes_in_registry.to_string(),
                &underperforming_nodes.to_string(),
            ])
            .unwrap();
        }

        wtr.flush().unwrap();
        Ok(())
    }

    /// Create node metrics CSV file
    fn create_node_metrics_csv(&self, output_dir: &str, daily_rewards: &[NodeProviderRewards]) -> Result<()> {
        // Collect all unique nodes across all days
        let mut nodes_data: BTreeMap<String, Vec<(RewardsDayUtc, String, String, String, String, Decimal)>> = BTreeMap::new();

        for daily_reward in daily_rewards {
            let day_utc = daily_reward.day_utc.unwrap();
            let rewards_day_utc = Self::convert_day_utc(&day_utc);

            if let Some(rewards) = &daily_reward.node_provider_rewards {
                for node_result in &rewards.nodes_results {
                    if let Some(node_id) = &node_result.node_id {
                        let node_id_str = node_id.to_string();
                        let performance_multiplier = Decimal::try_from(node_result.performance_multiplier_percent.as_ref().unwrap().clone()).unwrap();

                        nodes_data.entry(node_id_str).or_insert_with(Vec::new).push((
                            rewards_day_utc,
                            node_result.node_reward_type.clone().unwrap_or("unknown".to_string()),
                            node_result.region.clone().unwrap_or("unknown".to_string()),
                            node_result.dc_id.clone().unwrap_or("unknown".to_string()),
                            performance_multiplier.to_string(),
                            performance_multiplier,
                        ));
                    }
                }
            }
        }

        let filename = format!("{}/node_metrics.csv", output_dir);
        let mut wtr = Writer::from_path(&filename).unwrap();
        wtr.write_record(&["node_id", "day_utc", "node_type", "region", "dc_id", "performance_multiplier_percent"])
            .unwrap();

        for (node_id, metrics) in nodes_data {
            for (day_utc, node_type, region, dc_id, performance_multiplier_str, _) in metrics {
                wtr.write_record(&[&node_id, &day_utc.to_string(), &node_type, &region, &dc_id, &performance_multiplier_str])
                    .unwrap();
            }
        }

        wtr.flush().unwrap();
        Ok(())
    }
}
