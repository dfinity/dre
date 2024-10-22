use crate::{
    computation_logger::Entry,
    types::{NodeProviderRewards, RewardsMultiplierStats},
};
use candid::Principal;
use ic_base_types::PrincipalId;
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardsTable};
use itertools::Itertools;
use num_traits::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{self, BTreeMap, HashMap};

use crate::{
    chrono_utils::DateTimeRange,
    computation_logger::{Operation, RewardsComputationLogger},
    stable_memory::{self, RegionNodeTypeCategory},
    types::{DailyNodeMetrics, NodeMetadata},
};

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

const RF: &str = "Linear Reduction factor";

pub struct RewardsManager {
    logger: RewardsComputationLogger,
}

impl RewardsManager {
    pub fn new() -> Self {
        Self {
            logger: RewardsComputationLogger::new(),
        }
    }

    /// Calculates the rewards reduction based on the failure rate.
    ///
    /// # Arguments
    ///
    /// * `failure_rate` - A reference to a `Decimal` value representing the failure rate.
    ///
    /// # Returns
    ///
    /// * A `Decimal` value representing the rewards reduction, where:
    ///   - `0` indicates no reduction (failure rate below the minimum threshold),
    ///   - `1` indicates maximum reduction (failure rate above the maximum threshold),
    ///   - A value between `0` and `1` represents a proportional reduction based on the failure rate.
    ///
    /// # Explanation
    ///
    /// 1. The function checks if the provided `failure_rate` is below the `MIN_FAILURE_RATE` -> no reduction in rewards.
    ///
    /// 2. It then checks if the `failure_rate` is above the `MAX_FAILURE_RATE` -> maximum reduction in rewards.
    ///
    /// 3. If the `failure_rate` is within the defined range (`MIN_FAILURE_RATE` to `MAX_FAILURE_RATE`),
    ///    the function calculates the reduction proportionally.
    fn rewards_reduction_percent(&mut self, failure_rate: &Decimal) -> Decimal {
        if failure_rate < &MIN_FAILURE_RATE {
            self.logger.execute(
                &format!(
                    "No Reduction applied because {} is less than {} failure rate.\n{}",
                    failure_rate.round_dp(4),
                    MIN_FAILURE_RATE,
                    RF
                ),
                Operation::Set(dec!(0)),
            )
        } else if failure_rate > &MAX_FAILURE_RATE {
            self.logger.execute(
                &format!(
                    "Max reduction applied because {} is over {} failure rate.\n{}",
                    failure_rate.round_dp(4),
                    MAX_FAILURE_RATE,
                    RF
                ),
                Operation::Set(dec!(0.8)),
            )
        } else {
            let y_change = self
                .logger
                .execute("Linear Reduction Y change", Operation::Subtract(*failure_rate, MIN_FAILURE_RATE));
            let x_change = self
                .logger
                .execute("Linear Reduction X change", Operation::Subtract(MAX_FAILURE_RATE, MIN_FAILURE_RATE));

            let m = self.logger.execute("Compute m", Operation::Divide(y_change, x_change));

            self.logger.execute(RF, Operation::Multiply(m, dec!(0.8)))
        }
    }

    /// Compute rewards percent
    ///
    /// Computes the rewards percentage based on the overall failure rate in the period.
    ///
    /// # Arguments
    ///
    /// * `daily_metrics` - A slice of `DailyNodeMetrics` structs, where each struct represents the metrics for a single day.
    ///
    /// # Returns
    ///
    /// * A `RewardsComputationResult`.
    ///
    /// # Explanation
    ///
    /// 1. The function iterates through each day's metrics, summing up the `daily_failed` and `daily_total` blocks across all days.
    /// 2. The `overall_failure_rate` is calculated by dividing the `overall_failed` blocks by the `overall_total` blocks.
    /// 3. The `rewards_reduction` function is applied to `overall_failure_rate`.
    /// 3. Finally, the rewards percentage to be distrubuted to the node is computed.
    fn node_rewards_multiplier(&mut self, daily_metrics: &[DailyNodeMetrics], total_days: u64) -> (Decimal, RewardsMultiplierStats) {
        let total_days = Decimal::from(total_days);

        let days_assigned = self
            .logger
            .execute("Assigned Days In Period", Operation::Set(Decimal::from(daily_metrics.len())));
        let days_unassigned = self
            .logger
            .execute("Unassigned Days In Period", Operation::Subtract(total_days, days_assigned));

        let daily_failed = daily_metrics.iter().map(|metrics| metrics.num_blocks_failed.into()).collect_vec();
        let daily_proposed = daily_metrics.iter().map(|metrics| metrics.num_blocks_proposed.into()).collect_vec();

        let overall_failed = self.logger.execute("Computing Total Failed Blocks", Operation::Sum(daily_failed));
        let overall_proposed = self.logger.execute("Computing Total Proposed Blocks", Operation::Sum(daily_proposed));
        let overall_total = self
            .logger
            .execute("Computing Total Blocks", Operation::Sum(vec![overall_failed, overall_proposed]));
        let overall_failure_rate = self.logger.execute(
            "Computing Total Failure Rate",
            if overall_total > dec!(0) {
                Operation::Divide(overall_failed, overall_total)
            } else {
                Operation::Set(dec!(0))
            },
        );

        let rewards_reduction = self.rewards_reduction_percent(&overall_failure_rate);
        let rewards_multiplier_unassigned = self.logger.execute("Reward Multiplier Unassigned Days", Operation::Set(dec!(1)));
        let rewards_multiplier_assigned = self
            .logger
            .execute("Reward Multiplier Assigned Days", Operation::Subtract(dec!(1), rewards_reduction));
        let assigned_days_factor = self
            .logger
            .execute("Assigned Days Factor", Operation::Multiply(days_assigned, rewards_multiplier_assigned));
        let unassigned_days_factor = self.logger.execute(
            "Unassigned Days Factor",
            Operation::Multiply(days_unassigned, rewards_multiplier_unassigned),
        );
        let rewards_multiplier = self.logger.execute(
            "Average reward multiplier",
            Operation::Divide(assigned_days_factor + unassigned_days_factor, total_days),
        );

        let rewards_multiplier_stats = RewardsMultiplierStats {
            days_assigned: days_assigned.to_u64().unwrap(),
            days_unassigned: days_unassigned.to_u64().unwrap(),
            rewards_reduction: rewards_reduction.to_f64().unwrap(),
            blocks_failed: overall_failed.to_u64().unwrap(),
            blocks_proposed: overall_proposed.to_u64().unwrap(),
            blocks_total: overall_total.to_u64().unwrap(),
            failure_rate: overall_failure_rate.to_f64().unwrap(),
        };

        (rewards_multiplier, rewards_multiplier_stats)
    }

    fn node_provider_rewards(
        &mut self,
        assigned_multipliers: &collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>>,
        rewardable_nodes: &collections::BTreeMap<RegionNodeTypeCategory, u32>,
        rewards_table: &NodeRewardsTable,
    ) -> NodeProviderRewards {
        let mut rewards_xdr_total = dec!(0);
        let mut rewards_xdr_no_reduction_total = dec!(0);

        let mut type3_coefficients_rewards: HashMap<String, (Vec<Decimal>, Vec<Decimal>)> = HashMap::new();
        let mut other_rewards: HashMap<RegionNodeTypeCategory, Decimal> = HashMap::new();

        // Extract coefficients and rewards for type3* nodes in all regions
        for ((region, node_type), node_count) in rewardable_nodes {
            let rate = match rewards_table.get_rate(region, node_type) {
                Some(rate) => rate,
                None => {
                    self.logger.add_entry(Entry::RateNotFoundInRewardTable {
                        node_type: node_type.clone(),
                        region: region.clone(),
                    });

                    NodeRewardRate {
                        xdr_permyriad_per_node_per_month: 1,
                        reward_coefficient_percent: Some(100),
                    }
                }
            };
            let base_rewards = Decimal::from(rate.xdr_permyriad_per_node_per_month);

            if node_type.starts_with("type3") && *node_count > 0 {
                let coeff = Decimal::from(rate.reward_coefficient_percent.unwrap_or(80)) / dec!(100);
                let coefficients = vec![coeff; *node_count as usize];
                let rewards = vec![base_rewards; *node_count as usize];
                let region_key = region.splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");

                self.logger.add_entry(Entry::Type3NodesCoefficientsRewards {
                    node_type: node_type.clone(),
                    region: region.clone(),
                    coeff,
                    base_rewards,
                });

                type3_coefficients_rewards
                    .entry(region_key)
                    .and_modify(|(entry_coefficients, entry_rewards)| {
                        entry_coefficients.extend(&coefficients);
                        entry_rewards.extend(&rewards);
                    })
                    .or_insert((coefficients, rewards));
            } else {
                self.logger.add_entry(Entry::OtherNodesRewards {
                    node_type: node_type.clone(),
                    region: region.clone(),
                    base_rewards,
                });
                other_rewards.insert((region.clone(), node_type.clone()), base_rewards);
            }
        }

        // Compute node rewards for type3* nodes in all regions
        let type3_rewards: HashMap<String, Decimal> = type3_coefficients_rewards
            .clone()
            .into_iter()
            .map(|(region, (coefficients, rewards))| {
                let mut running_coefficient = dec!(1);
                let mut region_rewards = dec!(0);

                let coefficients_sum = self.logger.execute(
                    &format!("Coefficients sum in region {} for type3* nodes", region),
                    Operation::Sum(coefficients.clone()),
                );
                let coefficients_avg = self.logger.execute(
                    &format!("Coefficients avg in region {} for type3* nodes", region),
                    Operation::Divide(coefficients_sum, Decimal::from(coefficients.len())),
                );

                let rewards_sum = self.logger.execute(
                    &format!("Rewards sum in region {} for type3* nodes", region),
                    Operation::Sum(rewards.clone()),
                );
                let rewards_avg = self.logger.execute(
                    &format!("Rewards avg in region {} for type3* nodes", region),
                    Operation::Divide(rewards_sum, Decimal::from(rewards.len())),
                );

                for _ in 0..rewards.len() {
                    region_rewards += rewards_avg * running_coefficient;
                    running_coefficient *= coefficients_avg;
                }
                let region_rewards_avg = self.logger.execute(
                    &format!(
                        "Computing rewards average after coefficient reduction in region {} for type3* nodes",
                        region
                    ),
                    Operation::Divide(region_rewards, Decimal::from(rewards.len())),
                );

                (region, region_rewards_avg)
            })
            .collect();

        // Compute total rewards with reductions
        for ((region, node_type), node_count) in rewardable_nodes {
            let mut rewards_xdr = dec!(0);
            let mut rewards_multipliers = assigned_multipliers.get(&(region.clone(), node_type.clone())).unwrap_or(&vec![]).clone();
            rewards_multipliers.resize(*node_count as usize, dec!(1));

            self.logger.execute(
                &format!(
                    "Rewards multipliers len for nodes in region {} with type {}: {:?}\n",
                    &region, &node_type, rewards_multipliers
                ),
                Operation::Set(Decimal::from(rewards_multipliers.len())),
            );

            for multiplier in rewards_multipliers {
                if node_type.starts_with("type3") {
                    let region_key = region.as_str().splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");
                    let xdr_permyriad_avg_based = type3_rewards.get(&region_key).expect("Type3 rewards should have been filled already");

                    rewards_xdr_no_reduction_total += *xdr_permyriad_avg_based;
                    rewards_xdr += *xdr_permyriad_avg_based * multiplier;
                } else {
                    let xdr_permyriad = other_rewards.get(&(region.clone(), node_type.clone())).expect("Rewards already filled");
                    rewards_xdr_no_reduction_total += xdr_permyriad;
                    rewards_xdr += xdr_permyriad * multiplier;
                }
            }

            self.logger.execute(
                &format!(
                    "Rewards contribution XDR * 10'000 for nodes in region {} with type: {}\n",
                    region, node_type
                ),
                Operation::Set(rewards_xdr),
            );

            rewards_xdr_total += rewards_xdr;
        }

        self.logger.execute("Total rewards XDR * 10'000\n", Operation::Set(rewards_xdr_total));

        NodeProviderRewards {
            rewards_xdr_permyriad: rewards_xdr_total.to_u64().unwrap(),
            rewards_xdr_permyriad_no_reduction: rewards_xdr_no_reduction_total.to_u64().unwrap(),
        }
    }

    pub fn compute_node_providers_rewards(
        &mut self,
        node_providers_rewardables: BTreeMap<(Principal, RegionNodeTypeCategory), u32>,
        assigned_nodes_performance: BTreeMap<Principal, (NodeMetadata, Vec<DailyNodeMetrics>)>,
        rewarding_period: DateTimeRange,
        rewards_table: NodeRewardsTable,
    ) {
        let total_days = rewarding_period.days_between();
        let rewards_ts = rewarding_period.end_timestamp_nanos();
        let node_providers = node_providers_rewardables
            .keys()
            .map(|(node_provider_principal, _)| PrincipalId::from(*node_provider_principal))
            .unique()
            .collect_vec();

        for node_provider in node_providers {
            self.logger.add_entry(Entry::RewardsForNodeProvider(node_provider));

            let mut assigned_multipliers: BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();

            for (node_id, (metadata, daily_metrics)) in &assigned_nodes_performance {
                if metadata.node_provider_id == node_provider.0 {
                    self.logger.add_entry(Entry::RewardsMultiplier(PrincipalId::from(*node_id)));

                    let (multiplier, multiplier_stats) = self.node_rewards_multiplier(daily_metrics, total_days);
                    let region_node_type = (metadata.region.clone(), metadata.node_type.clone());

                    stable_memory::store_node_rewards_multiplier(
                        rewards_ts,
                        node_provider.0,
                        *node_id,
                        multiplier.to_u64().unwrap(),
                        multiplier_stats,
                    );
                    assigned_multipliers.entry(region_node_type).or_default().push(multiplier);
                }
            }

            let rewardables = node_providers_rewardables
                .iter()
                .filter(|((node_provider_principal, _), _)| node_provider_principal == &node_provider.0)
                .map(|((_, region_node_type), count)| (region_node_type.clone(), *count))
                .collect();

            let node_provider_rewards = self.node_provider_rewards(&assigned_multipliers, &rewardables, &rewards_table);

            ic_cdk::println!("Storing rewards for node provider {}", node_provider);
            stable_memory::store_node_provider_rewards(rewards_ts, node_provider.0, node_provider_rewards);
            
            ic_cdk::println!("Storing logs for node provider {}", node_provider);
            stable_memory::store_node_provider_logs(rewards_ts, node_provider.0, self.logger.get_log());

            self.logger.flush_log_entries();
        }
    }
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use ic_protobuf::registry::node_rewards::v2::NodeRewardRates;
    use itertools::Itertools;

    use super::*;

    #[derive(Clone)]
    struct MockedMetrics {
        days: u64,
        proposed_blocks: u64,
        failed_blocks: u64,
    }

    impl MockedMetrics {
        fn new(days: u64, proposed_blocks: u64, failed_blocks: u64) -> Self {
            MockedMetrics {
                days,
                proposed_blocks,
                failed_blocks,
            }
        }
    }

    fn daily_mocked_metrics(metrics: Vec<MockedMetrics>) -> Vec<DailyNodeMetrics> {
        let subnet = Principal::anonymous();
        let mut i = 0;

        metrics
            .into_iter()
            .flat_map(|mocked_metrics: MockedMetrics| {
                (0..mocked_metrics.days).map(move |_| {
                    i += 1;
                    DailyNodeMetrics::new(i, subnet, mocked_metrics.proposed_blocks, mocked_metrics.failed_blocks)
                })
            })
            .collect_vec()
    }

    fn mocked_rewards_table() -> NodeRewardsTable {
        let mut rates_outer: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
        let mut rates_inner: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
        let mut table: BTreeMap<String, NodeRewardRates> = BTreeMap::new();

        let rate_outer = NodeRewardRate {
            xdr_permyriad_per_node_per_month: 1000,
            reward_coefficient_percent: Some(97),
        };

        let rate_inner = NodeRewardRate {
            xdr_permyriad_per_node_per_month: 1500,
            reward_coefficient_percent: Some(95),
        };

        rates_outer.insert("type0".to_string(), rate_outer.clone());
        rates_outer.insert("type1".to_string(), rate_outer.clone());
        rates_outer.insert("type3".to_string(), rate_outer);

        rates_inner.insert("type3.1".to_string(), rate_inner);

        table.insert("A,B,C".to_string(), NodeRewardRates { rates: rates_inner });
        table.insert("A,B".to_string(), NodeRewardRates { rates: rates_outer });

        NodeRewardsTable { table }
    }

    #[test]
    fn test_rewards_percent() {
        let mut rewards_manager = RewardsManager::new();
        // Overall failed = 130 Overall total = 500 Failure rate = 0.26
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(20, 6, 4), MockedMetrics::new(25, 10, 2)]);
        let (result, _) = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(0.744));

        // Overall failed = 45 Overall total = 450 Failure rate = 0.1
        // rewards_reduction = 0.0
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 400, 20),
            MockedMetrics::new(1, 5, 25), // no penalty
        ]);
        let (result, _) = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(1.0));

        // Overall failed = 5 Overall total = 10 Failure rate = 0.5
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5), // no penalty
        ]);
        let (result, _) = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(0.36));
    }

    #[test]
    fn test_rewards_percent_max_reduction() {
        let mut rewards_manager = RewardsManager::new();
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 5, 95), // max failure rate
        ]);
        let (result, _) = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(0.2));
    }

    #[test]
    fn test_rewards_percent_min_reduction() {
        let mut rewards_manager = RewardsManager::new();
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 9, 1), // min failure rate
        ]);
        let (result, _) = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(1.0));
    }

    #[test]
    fn test_same_rewards_percent_if_gaps_no_penalty() {
        let mut rewards_manager = RewardsManager::new();
        let gap = MockedMetrics::new(1, 10, 0);

        let daily_metrics_mid_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![MockedMetrics::new(1, 6, 4), gap.clone(), MockedMetrics::new(1, 7, 3)]);

        let daily_metrics_left_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![gap.clone(), MockedMetrics::new(1, 6, 4), MockedMetrics::new(1, 7, 3)]);

        let daily_metrics_right_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![gap.clone(), MockedMetrics::new(1, 6, 4), MockedMetrics::new(1, 7, 3)]);

        assert_eq!(
            rewards_manager
                .node_rewards_multiplier(&daily_metrics_mid_gap, daily_metrics_mid_gap.len() as u64)
                .0,
            dec!(0.7866666666666666666666666667)
        );

        assert_eq!(
            rewards_manager
                .node_rewards_multiplier(&daily_metrics_mid_gap, daily_metrics_mid_gap.len() as u64)
                .0,
            rewards_manager
                .node_rewards_multiplier(&daily_metrics_left_gap, daily_metrics_left_gap.len() as u64)
                .0
        );
        assert_eq!(
            rewards_manager
                .node_rewards_multiplier(&daily_metrics_right_gap, daily_metrics_right_gap.len() as u64)
                .0,
            rewards_manager
                .node_rewards_multiplier(&daily_metrics_left_gap, daily_metrics_left_gap.len() as u64)
                .0
        );
    }

    #[test]
    fn test_same_rewards_if_reversed() {
        let mut rewards_manager = RewardsManager::new();
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5),
            MockedMetrics::new(5, 6, 4),
            MockedMetrics::new(25, 10, 0),
        ]);

        let mut daily_metrics = daily_metrics.clone();
        let result = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        daily_metrics.reverse();
        let result_rev = rewards_manager.node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);

        assert_eq!(result.0, dec!(1.0));
        assert_eq!(result_rev.0, result.0);
    }

    #[test]
    fn test_np_rewards_other_type() {
        let mut rewards_manager = RewardsManager::new();
        let mut assigned_multipliers: collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = BTreeMap::new();

        assigned_multipliers.insert(("A,B,C".to_string(), "type0".to_string()), vec![dec!(0.5), dec!(0.5)]);
        rewardable_nodes.insert(("A,B,C".to_string(), "type0".to_string()), 4);
        let node_rewards_table: NodeRewardsTable = mocked_rewards_table();
        let rewards = rewards_manager.node_provider_rewards(&assigned_multipliers, &rewardable_nodes, &node_rewards_table);

        // 4 nodes type0 1000 * 4 = 4000
        assert_eq!(rewards.rewards_xdr_permyriad_no_reduction, 4000);
        // 4 nodes type0 1000 * 1 + 1000 * 1 + 1000 * 0.5 + 1000 * 0.5 * 4 = 3000
        assert_eq!(rewards.rewards_xdr_permyriad, 3000);
    }

    #[test]
    fn test_np_rewards_type3_coeff() {
        let mut rewards_manager = RewardsManager::new();
        let mut assigned_multipliers: collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = BTreeMap::new();

        assigned_multipliers.insert(("A,B,C".to_string(), "type3.1".to_string()), vec![dec!(0.5)]);
        rewardable_nodes.insert(("A,B,C".to_string(), "type3.1".to_string()), 4);
        let node_rewards_table: NodeRewardsTable = mocked_rewards_table();
        let rewards = rewards_manager.node_provider_rewards(&assigned_multipliers, &rewardable_nodes, &node_rewards_table);

        // 4 nodes type3.1 avg rewards 1500 avg coefficient 0.95
        // 1500 * 1 + 1500 * 0.95 + 1500 * 0.95 * 0.95 + 1500 * 0.95 * 0.95 * 0.95
        assert_eq!(rewards.rewards_xdr_permyriad_no_reduction, 5564);

        // rewards coeff avg 5564/4=1391
        // 1391 * 0.5 + 1391 * 3 = 4868
        assert_eq!(rewards.rewards_xdr_permyriad, 4869);
    }

    #[test]
    fn test_np_rewards_type3_mix() {
        let mut rewards_manager = RewardsManager::new();
        let mut assigned_multipliers: collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = BTreeMap::new();

        assigned_multipliers.insert(("A,B,D".to_string(), "type3".to_string()), vec![dec!(0.5)]);

        // This will take rates from outer
        rewardable_nodes.insert(("A,B,D".to_string(), "type3".to_string()), 2);

        // This will take rates from inner
        rewardable_nodes.insert(("A,B,C".to_string(), "type3.1".to_string()), 2);

        let node_rewards_table: NodeRewardsTable = mocked_rewards_table();
        let rewards = rewards_manager.node_provider_rewards(&assigned_multipliers, &rewardable_nodes, &node_rewards_table);

        // 4 nodes type3* avg rewards 1250 avg coefficient 0.96
        // 1250 * 1 + 1250 * 0.96 + 1250 * 0.96^2 + 1250 * 0.96^3
        assert_eq!(rewards.rewards_xdr_permyriad_no_reduction, 4707);

        // rewards coeff avg 4707/4 = 1176.75
        // 1176.75 * 0.5 + 1176.75 * 3
        assert_eq!(rewards.rewards_xdr_permyriad, 4119);
    }
}
