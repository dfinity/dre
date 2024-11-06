use ic_base_types::PrincipalId;
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardsTable};
use itertools::Itertools;
use lazy_static::lazy_static;
use num_traits::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::{
    collections::{self, BTreeMap, HashMap},
    mem,
    sync::{Arc, RwLock},
};

use crate::{
    v1_logs::{LogEntry, Operation, RewardsPerNodeProviderLog},
    v1_types::{
        DailyNodeMetrics, MultiplierStats, Node, NodeMultiplierStats, RegionNodeTypeCategory, RewardablesWithMetrics, Rewards, RewardsPerNodeProvider,
    },
};

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

const RF: &str = "Linear Reduction factor";

lazy_static! {
    static ref LOGGER: Arc<RwLock<RewardsPerNodeProviderLog>> = Arc::new(RwLock::new(RewardsPerNodeProviderLog::default()));
}

fn logger_write() -> std::sync::RwLockWriteGuard<'static, RewardsPerNodeProviderLog> {
    LOGGER.write().unwrap()
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
fn rewards_reduction_percent(failure_rate: &Decimal) -> Decimal {
    if failure_rate < &MIN_FAILURE_RATE {
        logger_write().execute(
            &format!(
                "No Reduction applied because {} is less than {} failure rate.\n{}",
                failure_rate.round_dp(4),
                MIN_FAILURE_RATE,
                RF
            ),
            Operation::Set(dec!(0)),
        )
    } else if failure_rate > &MAX_FAILURE_RATE {
        logger_write().execute(
            &format!(
                "Max reduction applied because {} is over {} failure rate.\n{}",
                failure_rate.round_dp(4),
                MAX_FAILURE_RATE,
                RF
            ),
            Operation::Set(dec!(0.8)),
        )
    } else {
        let y_change = logger_write().execute("Linear Reduction Y change", Operation::Subtract(*failure_rate, MIN_FAILURE_RATE));
        let x_change = logger_write().execute("Linear Reduction X change", Operation::Subtract(MAX_FAILURE_RATE, MIN_FAILURE_RATE));

        let m = logger_write().execute("Compute m", Operation::Divide(y_change, x_change));

        logger_write().execute(RF, Operation::Multiply(m, dec!(0.8)))
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
fn node_rewards_multiplier(daily_metrics: &[DailyNodeMetrics], total_days: u64) -> (Decimal, MultiplierStats) {
    let total_days = Decimal::from(total_days);

    let days_assigned = logger_write().execute("Assigned Days In Period", Operation::Set(Decimal::from(daily_metrics.len())));
    let days_unassigned = logger_write().execute("Unassigned Days In Period", Operation::Subtract(total_days, days_assigned));

    let daily_failed = daily_metrics.iter().map(|metrics| metrics.num_blocks_failed.into()).collect_vec();
    let daily_proposed = daily_metrics.iter().map(|metrics| metrics.num_blocks_proposed.into()).collect_vec();

    let overall_failed = logger_write().execute("Computing Total Failed Blocks", Operation::Sum(daily_failed));
    let overall_proposed = logger_write().execute("Computing Total Proposed Blocks", Operation::Sum(daily_proposed));
    let overall_total = logger_write().execute("Computing Total Blocks", Operation::Sum(vec![overall_failed, overall_proposed]));
    let overall_failure_rate = logger_write().execute(
        "Computing Total Failure Rate",
        if overall_total > dec!(0) {
            Operation::Divide(overall_failed, overall_total)
        } else {
            Operation::Set(dec!(0))
        },
    );

    let rewards_reduction = rewards_reduction_percent(&overall_failure_rate);
    let rewards_multiplier_unassigned = logger_write().execute("Reward Multiplier Unassigned Days", Operation::Set(dec!(1)));
    let rewards_multiplier_assigned = logger_write().execute("Reward Multiplier Assigned Days", Operation::Subtract(dec!(1), rewards_reduction));
    let assigned_days_factor = logger_write().execute("Assigned Days Factor", Operation::Multiply(days_assigned, rewards_multiplier_assigned));
    let unassigned_days_factor = logger_write().execute(
        "Unassigned Days Factor",
        Operation::Multiply(days_unassigned, rewards_multiplier_unassigned),
    );
    let rewards_multiplier = logger_write().execute(
        "Average reward multiplier",
        Operation::Divide(assigned_days_factor + unassigned_days_factor, total_days),
    );

    let rewards_multiplier_stats = MultiplierStats {
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
    assigned_multipliers: &collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>>,
    rewardable_nodes: &collections::BTreeMap<RegionNodeTypeCategory, u32>,
    rewards_table: &NodeRewardsTable,
) -> Rewards {
    let mut rewards_xdr_total = dec!(0);
    let mut rewards_xdr_no_reduction_total = dec!(0);

    let mut type3_coefficients_rewards: HashMap<String, (Vec<Decimal>, Vec<Decimal>)> = HashMap::new();
    let mut other_rewards: HashMap<RegionNodeTypeCategory, Decimal> = HashMap::new();

    // Extract coefficients and rewards for type3* nodes in all regions
    for ((region, node_type), node_count) in rewardable_nodes {
        let rate = match rewards_table.get_rate(region, node_type) {
            Some(rate) => rate,
            None => {
                logger_write().add_entry(LogEntry::RateNotFoundInRewardTable {
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

            logger_write().add_entry(LogEntry::Type3NodesCoefficientsRewards {
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
            logger_write().add_entry(LogEntry::OtherNodesRewards {
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

            let coefficients_sum = logger_write().execute(
                &format!("Coefficients sum in region {} for type3* nodes", region),
                Operation::Sum(coefficients.clone()),
            );
            let coefficients_avg = logger_write().execute(
                &format!("Coefficients avg in region {} for type3* nodes", region),
                Operation::Divide(coefficients_sum, Decimal::from(coefficients.len())),
            );

            let rewards_sum = logger_write().execute(
                &format!("Rewards sum in region {} for type3* nodes", region),
                Operation::Sum(rewards.clone()),
            );
            let rewards_avg = logger_write().execute(
                &format!("Rewards avg in region {} for type3* nodes", region),
                Operation::Divide(rewards_sum, Decimal::from(rewards.len())),
            );

            for _ in 0..rewards.len() {
                region_rewards += rewards_avg * running_coefficient;
                running_coefficient *= coefficients_avg;
            }
            let region_rewards_avg = logger_write().execute(
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

        logger_write().execute(
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

        logger_write().execute(
            &format!(
                "Rewards contribution XDR permyriad for nodes in region {} with type: {}\n",
                region, node_type
            ),
            Operation::Set(rewards_xdr),
        );

        rewards_xdr_total += rewards_xdr;
    }

    logger_write().execute("Total rewards XDR permyriad\n", Operation::Set(rewards_xdr_total));

    Rewards {
        xdr_permyriad: rewards_xdr_total.to_u64().unwrap(),
        xdr_permyriad_no_reduction: rewards_xdr_no_reduction_total.to_u64().unwrap(),
    }
}

fn node_providers_rewardables(
    nodes: &[Node],
    nodes_assigned_metrics: &BTreeMap<PrincipalId, Vec<DailyNodeMetrics>>,
) -> BTreeMap<PrincipalId, RewardablesWithMetrics> {
    let mut node_provider_rewardables: BTreeMap<PrincipalId, RewardablesWithMetrics> = BTreeMap::new();

    nodes.iter().for_each(|node| {
        let (rewardable_nodes, assigned_nodes_metrics) = node_provider_rewardables.entry(node.node_provider_id).or_default();

        let nodes_count = rewardable_nodes.entry((node.region.clone(), node.node_type.clone())).or_default();
        *nodes_count += 1;

        if let Some(daily_metrics) = nodes_assigned_metrics.get(&node.node_id) {
            assigned_nodes_metrics.insert(node.clone(), daily_metrics.clone());
        }
    });

    node_provider_rewardables
}

pub fn calculate_rewards(
    rewards_table: &NodeRewardsTable,
    days_in_period: u64,
    nodes_in_period: &[Node],
    assigned_nodes_metrics: &BTreeMap<PrincipalId, Vec<DailyNodeMetrics>>,
) -> Result<RewardsPerNodeProvider, String> {
    let mut rewards_per_node_provider = BTreeMap::new();
    let mut computation_log = BTreeMap::new();
    let node_provider_rewardables = node_providers_rewardables(nodes_in_period, assigned_nodes_metrics);

    for (node_provider_id, (rewardable_nodes, assigned_nodes_metrics)) in node_provider_rewardables {
        let mut assigned_multipliers: BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut nodes_multiplier_stats: Vec<NodeMultiplierStats> = Vec::new();

        logger_write().add_entry(LogEntry::RewardsForNodeProvider(node_provider_id));

        for (node, daily_metrics) in assigned_nodes_metrics {
            logger_write().add_entry(LogEntry::RewardsMultiplier(node.node_id));
            let (multiplier, multiplier_stats) = node_rewards_multiplier(&daily_metrics, days_in_period);

            nodes_multiplier_stats.push((node.node_id, multiplier_stats));
            assigned_multipliers
                .entry((node.region.clone(), node.node_type.clone()))
                .or_default()
                .push(multiplier);
        }
        let rewards = node_provider_rewards(&assigned_multipliers, &rewardable_nodes, rewards_table);
        let node_provider_log = mem::take(&mut *logger_write());

        computation_log.insert(node_provider_id, node_provider_log);
        rewards_per_node_provider.insert(node_provider_id, (rewards, nodes_multiplier_stats));
    }

    Ok(RewardsPerNodeProvider {
        rewards_per_node_provider,
        computation_log,
    })
}

#[cfg(test)]
mod tests {
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
        let subnet = PrincipalId::new_anonymous();
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

        rates_outer.insert("type0".to_string(), rate_outer);
        rates_outer.insert("type1".to_string(), rate_outer);
        rates_outer.insert("type3".to_string(), rate_outer);

        rates_inner.insert("type3.1".to_string(), rate_inner);

        table.insert("A,B,C".to_string(), NodeRewardRates { rates: rates_inner });
        table.insert("A,B".to_string(), NodeRewardRates { rates: rates_outer });

        NodeRewardsTable { table }
    }

    #[test]
    fn test_rewards_percent() {
        // Overall failed = 130 Overall total = 500 Failure rate = 0.26
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(20, 6, 4), MockedMetrics::new(25, 10, 2)]);
        let (result, _) = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(0.744));

        // Overall failed = 45 Overall total = 450 Failure rate = 0.1
        // rewards_reduction = 0.0
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 400, 20),
            MockedMetrics::new(1, 5, 25), // no penalty
        ]);
        let (result, _) = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(1.0));

        // Overall failed = 5 Overall total = 10 Failure rate = 0.5
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5), // no penalty
        ]);
        let (result, _) = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(0.36));
    }

    #[test]
    fn test_rewards_percent_max_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 5, 95), // max failure rate
        ]);
        let (result, _) = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(0.2));
    }

    #[test]
    fn test_rewards_percent_min_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 9, 1), // min failure rate
        ]);
        let (result, _) = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result, dec!(1.0));
    }

    #[test]
    fn test_same_rewards_percent_if_gaps_no_penalty() {
        let gap = MockedMetrics::new(1, 10, 0);
        let daily_metrics_mid_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![MockedMetrics::new(1, 6, 4), gap.clone(), MockedMetrics::new(1, 7, 3)]);
        let daily_metrics_left_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![gap.clone(), MockedMetrics::new(1, 6, 4), MockedMetrics::new(1, 7, 3)]);
        let daily_metrics_right_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![gap.clone(), MockedMetrics::new(1, 6, 4), MockedMetrics::new(1, 7, 3)]);

        assert_eq!(
            node_rewards_multiplier(&daily_metrics_mid_gap, daily_metrics_mid_gap.len() as u64).0,
            dec!(0.7866666666666666666666666667)
        );

        assert_eq!(
            node_rewards_multiplier(&daily_metrics_mid_gap, daily_metrics_mid_gap.len() as u64).0,
            node_rewards_multiplier(&daily_metrics_left_gap, daily_metrics_left_gap.len() as u64).0
        );
        assert_eq!(
            node_rewards_multiplier(&daily_metrics_right_gap, daily_metrics_right_gap.len() as u64).0,
            node_rewards_multiplier(&daily_metrics_left_gap, daily_metrics_left_gap.len() as u64).0
        );
    }

    #[test]
    fn test_same_rewards_if_reversed() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5),
            MockedMetrics::new(5, 6, 4),
            MockedMetrics::new(25, 10, 0),
        ]);

        let mut daily_metrics = daily_metrics.clone();
        let result = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        daily_metrics.reverse();
        let result_rev = node_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);

        assert_eq!(result.0, dec!(1.0));
        assert_eq!(result_rev.0, result.0);
    }

    #[test]
    fn test_np_rewards_other_type() {
        let mut assigned_multipliers: collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = BTreeMap::new();

        assigned_multipliers.insert(("A,B,C".to_string(), "type0".to_string()), vec![dec!(0.5), dec!(0.5)]);
        rewardable_nodes.insert(("A,B,C".to_string(), "type0".to_string()), 4);
        let node_rewards_table: NodeRewardsTable = mocked_rewards_table();
        let rewards = node_provider_rewards(&assigned_multipliers, &rewardable_nodes, &node_rewards_table);

        // 4 nodes type0 1000 * 4 = 4000
        assert_eq!(rewards.xdr_permyriad_no_reduction, 4000);
        // 4 nodes type0 1000 * 1 + 1000 * 1 + 1000 * 0.5 + 1000 * 0.5 * 4 = 3000
        assert_eq!(rewards.xdr_permyriad, 3000);
    }

    #[test]
    fn test_np_rewards_type3_coeff() {
        let mut assigned_multipliers: collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = BTreeMap::new();

        assigned_multipliers.insert(("A,B,C".to_string(), "type3.1".to_string()), vec![dec!(0.5)]);
        rewardable_nodes.insert(("A,B,C".to_string(), "type3.1".to_string()), 4);
        let node_rewards_table: NodeRewardsTable = mocked_rewards_table();
        let rewards = node_provider_rewards(&assigned_multipliers, &rewardable_nodes, &node_rewards_table);

        // 4 nodes type3.1 avg rewards 1500 avg coefficient 0.95
        // 1500 * 1 + 1500 * 0.95 + 1500 * 0.95 * 0.95 + 1500 * 0.95 * 0.95 * 0.95
        assert_eq!(rewards.xdr_permyriad_no_reduction, 5564);

        // rewards coeff avg 5564/4=1391
        // 1391 * 0.5 + 1391 * 3 = 4868
        assert_eq!(rewards.xdr_permyriad, 4869);
    }

    #[test]
    fn test_np_rewards_type3_mix() {
        let mut assigned_multipliers: collections::BTreeMap<RegionNodeTypeCategory, Vec<Decimal>> = BTreeMap::new();
        let mut rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = BTreeMap::new();

        assigned_multipliers.insert(("A,B,D".to_string(), "type3".to_string()), vec![dec!(0.5)]);

        // This will take rates from outer
        rewardable_nodes.insert(("A,B,D".to_string(), "type3".to_string()), 2);

        // This will take rates from inner
        rewardable_nodes.insert(("A,B,C".to_string(), "type3.1".to_string()), 2);

        let node_rewards_table: NodeRewardsTable = mocked_rewards_table();
        let rewards = node_provider_rewards(&assigned_multipliers, &rewardable_nodes, &node_rewards_table);

        // 4 nodes type3* avg rewards 1250 avg coefficient 0.96
        // 1250 * 1 + 1250 * 0.96 + 1250 * 0.96^2 + 1250 * 0.96^3
        assert_eq!(rewards.xdr_permyriad_no_reduction, 4707);

        // rewards coeff avg 4707/4 = 1176.75
        // 1176.75 * 0.5 + 1176.75 * 3
        assert_eq!(rewards.xdr_permyriad, 4119);
    }
}
