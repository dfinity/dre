use candid::Principal;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use ic_nns_governance_api::pb::v1::MonthlyNodeProviderRewards;
use ic_protobuf::registry::node_rewards::{v2::NodeRewardRate, v2::NodeRewardsTable};
use ic_registry_keys::NODE_REWARDS_TABLE_KEY;
use itertools::Itertools;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{self, HashMap};
use trustworthy_node_metrics_types::types::{
    DailyNodeMetrics, NodeProviderRewards, NodeProviderRewardsComputation, NodeRewardsMultiplier, RewardsMultiplier,
};

use crate::{
    chrono_utils::DateTimeRange,
    computation_logger::{ComputationLogger, Operation, OperationExecutor},
    stable_memory::{self, RegionNodeTypeCategory},
};

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

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
fn rewards_reduction_percent(failure_rate: &Decimal) -> (Vec<OperationExecutor>, Decimal) {
    const RF: &str = "Linear Reduction factor";

    if failure_rate < &MIN_FAILURE_RATE {
        let (operation, result) = OperationExecutor::execute(
            &format!(
                "No Reduction applied because {} is less than {} failure rate.\n{}",
                failure_rate.round_dp(4),
                MIN_FAILURE_RATE,
                RF
            ),
            Operation::Set(dec!(0)),
        );
        (vec![operation], result)
    } else if failure_rate > &MAX_FAILURE_RATE {
        let (operation, result) = OperationExecutor::execute(
            &format!(
                "Max reduction applied because {} is over {} failure rate.\n{}",
                failure_rate.round_dp(4),
                MAX_FAILURE_RATE,
                RF
            ),
            Operation::Set(dec!(0.8)),
        );

        (vec![operation], result)
    } else {
        let (y_change_operation, y_change) =
            OperationExecutor::execute("Linear Reduction Y change", Operation::Subtract(*failure_rate, MIN_FAILURE_RATE));
        let (x_change_operation, x_change) =
            OperationExecutor::execute("Linear Reduction X change", Operation::Subtract(MAX_FAILURE_RATE, MIN_FAILURE_RATE));

        let (m_operation, m) = OperationExecutor::execute("Compute m", Operation::Divide(y_change, x_change));
        let (operation, result) = OperationExecutor::execute(RF, Operation::Multiply(m, dec!(0.8)));
        (vec![y_change_operation, x_change_operation, m_operation, operation], result)
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
fn compute_rewards_multiplier(daily_metrics: &[DailyNodeMetrics], total_days: u64) -> RewardsMultiplier {
    let mut computation_logger = ComputationLogger::new();

    let total_days = computation_logger.execute("Days In Period", Operation::Set(Decimal::from(total_days)));
    let days_assigned = computation_logger.execute("Assigned Days In Period", Operation::Set(Decimal::from(daily_metrics.len())));
    let days_unassigned = computation_logger.execute("Unassigned Days In Period", Operation::Subtract(total_days, days_assigned));

    let daily_failed = daily_metrics.iter().map(|metrics| metrics.num_blocks_failed.into()).collect_vec();
    let daily_proposed = daily_metrics.iter().map(|metrics| metrics.num_blocks_proposed.into()).collect_vec();

    let overall_failed = computation_logger.execute("Computing Total Failed Blocks", Operation::Sum(daily_failed));
    let overall_proposed = computation_logger.execute("Computing Total Proposed Blocks", Operation::Sum(daily_proposed));
    let overall_total = computation_logger.execute("Computing Total Blocks", Operation::Sum(vec![overall_failed, overall_proposed]));
    let overall_failure_rate = computation_logger.execute(
        "Computing Total Failure Rate",
        if overall_total > dec!(0) {
            Operation::Divide(overall_failed, overall_total)
        } else {
            Operation::Set(dec!(0))
        },
    );

    let (operations, rewards_reduction) = rewards_reduction_percent(&overall_failure_rate);
    computation_logger.add_executed(operations);
    let rewards_multiplier_unassigned = computation_logger.execute("Reward Multiplier Unassigned Days", Operation::Set(dec!(1)));
    let rewards_multiplier_assigned = computation_logger.execute("Reward Multiplier Assigned Days", Operation::Subtract(dec!(1), rewards_reduction));
    let assigned_days_factor = computation_logger.execute("Assigned Days Factor", Operation::Multiply(days_assigned, rewards_multiplier_assigned));
    let unassigned_days_factor = computation_logger.execute(
        "Unassigned Days Factor",
        Operation::Multiply(days_unassigned, rewards_multiplier_unassigned),
    );
    let rewards_multiplier = computation_logger.execute(
        "Average reward multiplier",
        Operation::Divide(assigned_days_factor + unassigned_days_factor, total_days),
    );

    RewardsMultiplier {
        days_assigned: days_assigned.to_u64().unwrap(),
        days_unassigned: days_unassigned.to_u64().unwrap(),
        rewards_multiplier: rewards_multiplier.to_f64().unwrap(),
        rewards_reduction: rewards_reduction.to_f64().unwrap(),
        blocks_failed: overall_failed.to_u64().unwrap(),
        blocks_proposed: overall_proposed.to_u64().unwrap(),
        blocks_total: overall_total.to_u64().unwrap(),
        failure_rate: overall_failure_rate.to_f64().unwrap(),
        computation_log: computation_logger.get_log(),
    }
}

fn get_node_rate(region: &String, node_type: &String) -> NodeRewardRate {
    match stable_memory::get_rate(region, node_type) {
        Some(rate) => rate,
        None => {
            ic_cdk::println!(
                "The Node Rewards Table does not have an entry for \
                     node type '{}' within region '{}' or parent region, defaulting to 1 xdr per month per node",
                node_type,
                region
            );
            NodeRewardRate {
                xdr_permyriad_per_node_per_month: 1,
                reward_coefficient_percent: Some(100),
            }
        }
    }
}

fn get_region_node_type_key(region: &str, node_type: &str) -> String {
    let region_key = region.splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");
    format!("{}:{}", region_key, if node_type.starts_with("type3") { "type3" } else { node_type })
}

fn coumpute_node_provider_rewards(
    nodes_multiplier: &[NodeRewardsMultiplier],
    rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32>,
) -> NodeProviderRewardsComputation {
    let mut rewards_xdr_total = dec!(0);
    let mut rewards_xdr_no_reduction_total = dec!(0);
    let mut computation_logger = ComputationLogger::new();

    let mut multipliers_group: HashMap<String, Vec<Decimal>> = HashMap::new();

    // 1.1 - Extract coefficients and rewards for type3* nodes in all regions

    let mut type3_coefficients_rewards: HashMap<String, (Vec<Decimal>, Vec<Decimal>)> = HashMap::new();

    for ((region, node_type), node_count) in rewardable_nodes.clone() {
        if node_type.starts_with("type3") && node_count > 0 {
            let rate = get_node_rate(&region, &node_type);
            let regional_coeff = Decimal::from(rate.reward_coefficient_percent.unwrap_or(80)) / dec!(100);
            let regional_rewards = Decimal::from(rate.xdr_permyriad_per_node_per_month);
            let coefficients = vec![regional_coeff; node_count as usize];
            let rewards = vec![regional_rewards; node_count as usize];

            computation_logger.execute(
                &format!(
                    "Setting count of nodes in region {} with type {}, with coefficient {}, with rewards {} XDRths\n",
                    region, node_type, regional_coeff, regional_rewards
                ),
                Operation::Set(Decimal::from(node_count)),
            );

            type3_coefficients_rewards
                .entry(region)
                .and_modify(|(entry_coefficients, entry_rewards)| {
                    entry_coefficients.extend(&coefficients);
                    entry_rewards.extend(&rewards);
                })
                .or_insert((coefficients, rewards));
        }
    }

    // 1.2 - Compute node rewards for type3* nodes in all regions

    let type3_rewards: HashMap<String, Decimal> = type3_coefficients_rewards
        .clone()
        .into_iter()
        .map(|(region, (coefficients, rewards))| {
            let mut running_coefficient = dec!(1);
            let mut region_rewards = dec!(0);
            let coefficients_avg = computation_logger.execute(
                &format!("Coefficient average in region {} for type3* nodes\n", region),
                Operation::Set(coefficients.iter().fold(Decimal::zero(), |acc, val| acc + val) / Decimal::from(coefficients.len())),
            );
            let rewards_avg = computation_logger.execute(
                &format!("Rewards average in region {} for type3* nodes\n", region),
                Operation::Set(rewards.iter().fold(Decimal::zero(), |acc, val| acc + val) / Decimal::from(rewards.len())),
            );

            for _ in 0..rewards.len() {
                region_rewards += rewards_avg * running_coefficient;
                running_coefficient *= coefficients_avg;
            }
            let region_rewards_avg = computation_logger.execute(
                &format!(
                    "Computing rewards average after coefficient reduction in region {} for type3* nodes\n",
                    region
                ),
                Operation::Divide(region_rewards, Decimal::from(rewards.len())),
            );

            (region, region_rewards_avg)
        })
        .collect();

    // 2 - For assigned nodes map to region and node_type the reward multipliers
    for node in nodes_multiplier {
        let metadata = stable_memory::get_node_metadata(&node.node_id).expect("Node should have one node provider");
        let key = get_region_node_type_key(&metadata.region, &metadata.node_type);

        let rewards_multiplier = Decimal::from_f64(node.rewards_multiplier.rewards_multiplier).unwrap();

        computation_logger.execute(
            &format!(
                "Set rewards multiplier for Node {}, in region {} with type {}\n",
                node.node_id, metadata.region, metadata.node_type
            ),
            Operation::Set(Decimal::from_f64(node.rewards_multiplier.rewards_multiplier).unwrap()),
        );

        multipliers_group.entry(key).or_default().push(rewards_multiplier);
    }

    // 3 - Now compute total rewards with reductions

    for ((region, node_type), node_count) in rewardable_nodes {
        // Fetch the key used by all node_types to get the multiplier
        let key = get_region_node_type_key(&region, &node_type);
        let rewards_multipliers = multipliers_group.entry(key).or_default();
        let mut rewards_xdr = dec!(0);
        rewards_multipliers.resize(node_count as usize, dec!(1));

        computation_logger.execute(
            &format!(
                "Rewards multipliers len for nodes in region {} with type {}: {:?}\n",
                region, node_type, rewards_multipliers
            ),
            Operation::Set(Decimal::from(rewards_multipliers.len())),
        );

        for multiplier in rewards_multipliers {
            if node_type.starts_with("type3") {
                let xdr_permyriad_avg_based = type3_rewards.get(&region).expect("Type3 rewards should have been filled already");
                rewards_xdr_no_reduction_total += *xdr_permyriad_avg_based;
                rewards_xdr += *xdr_permyriad_avg_based * *multiplier;
            } else {
                let rate = get_node_rate(&region, &node_type);
                let xdr_permyriad = Decimal::from(rate.xdr_permyriad_per_node_per_month);
                rewards_xdr_no_reduction_total += xdr_permyriad;
                rewards_xdr += xdr_permyriad * *multiplier;
            }
        }

        computation_logger.execute(
            &format!("Rewards contribution XDRths for nodes in region {} with type: {}\n", region, node_type),
            Operation::Set(rewards_xdr),
        );

        rewards_xdr_total += rewards_xdr;
    }

    computation_logger.execute("Total rewards XDRths\n", Operation::Set(rewards_xdr_total));

    NodeProviderRewardsComputation {
        rewards_xdr: rewards_xdr_total.to_u64().unwrap(),
        rewards_xdr_no_reduction: rewards_xdr_no_reduction_total.to_u64().unwrap(),
        computation_log: computation_logger.get_log(),
    }
}

pub fn node_rewards_multiplier(node_ids: Vec<Principal>, rewarding_period: DateTimeRange) -> Vec<NodeRewardsMultiplier> {
    let total_days = rewarding_period.days_between();
    let nodes_metrics = stable_memory::get_metrics_range(
        rewarding_period.start_timestamp_nanos(),
        Some(rewarding_period.end_timestamp_nanos()),
        Some(&node_ids),
    );
    let mut daily_metrics: collections::BTreeMap<Principal, Vec<DailyNodeMetrics>> = collections::BTreeMap::new();

    for node_id in node_ids {
        daily_metrics.entry(node_id).or_default();
    }

    for ((ts, node_id), node_metrics_value) in nodes_metrics {
        let daily_node_metrics = DailyNodeMetrics::new(
            ts,
            node_metrics_value.subnet_assigned,
            node_metrics_value.num_blocks_proposed,
            node_metrics_value.num_blocks_failed,
        );

        daily_metrics.entry(node_id).or_default().push(daily_node_metrics);
    }

    daily_metrics
        .into_iter()
        .map(|(node_id, daily_node_metrics)| {
            let rewards_multiplier = compute_rewards_multiplier(&daily_node_metrics, total_days);
            let node_metadata = stable_memory::get_node_metadata(&node_id).expect("Node should have one node provider");
            let node_rate = get_node_rate(&node_metadata.region, &node_metadata.node_type);

            NodeRewardsMultiplier {
                node_id,
                daily_node_metrics,
                node_rate,
                rewards_multiplier,
            }
        })
        .collect_vec()
}

pub fn node_provider_rewards(node_provider_id: Principal, rewarding_period: DateTimeRange) -> NodeProviderRewards {
    let node_ids = stable_memory::get_node_principals(&node_provider_id);
    let rewardable_nodes: collections::BTreeMap<RegionNodeTypeCategory, u32> = stable_memory::get_rewardable_nodes(&node_provider_id);
    let latest_np_rewards = stable_memory::get_latest_node_providers_rewards();

    let nodes_multiplier = node_rewards_multiplier(node_ids, rewarding_period);
    let rewards_computation = coumpute_node_provider_rewards(&nodes_multiplier, rewardable_nodes);

    let rewards_xdr_old = latest_np_rewards
        .rewards
        .into_iter()
        .filter_map(|np_rewards| {
            if let Some(node_provider) = np_rewards.node_provider {
                if let Some(id) = node_provider.id {
                    if id.0 == node_provider_id {
                        return Some(np_rewards.amount_e8s);
                    }
                }
            }
            None
        })
        .next();

    NodeProviderRewards {
        node_provider_id,
        rewards_xdr: rewards_computation.rewards_xdr,
        rewards_xdr_no_reduction: rewards_computation.rewards_xdr_no_reduction,
        computation_log: rewards_computation.computation_log,
        rewards_xdr_old,
        ts_distribution: latest_np_rewards.timestamp,
        xdr_conversion_rate: latest_np_rewards.xdr_conversion_rate.and_then(|rate| rate.xdr_permyriad_per_icp),
        nodes_rewards: nodes_multiplier,
    }
}

/// Update node rewards table
pub async fn update_node_rewards_table() -> anyhow::Result<()> {
    let (rewards_table, _): (NodeRewardsTable, _) = ic_nns_common::registry::get_value(NODE_REWARDS_TABLE_KEY.as_bytes(), None).await?;
    for (region, rewards_rates) in rewards_table.table {
        stable_memory::insert_rewards_rates(region, rewards_rates)
    }

    Ok(())
}

/// Update recent node providers rewards
pub async fn update_recent_provider_rewards() -> anyhow::Result<()> {
    let (maybe_monthly_rewards,): (Option<MonthlyNodeProviderRewards>,) = ic_cdk::api::call::call(
        Principal::from(GOVERNANCE_CANISTER_ID),
        "get_most_recent_monthly_node_provider_rewards",
        (),
    )
    .await
    .map_err(|(code, msg)| {
        anyhow::anyhow!(
            "Error when calling get_most_recent_monthly_node_provider_rewards:\n Code:{:?}\nMsg:{}",
            code,
            msg
        )
    })?;

    if let Some(monthly_rewards) = maybe_monthly_rewards {
        let latest_np_rewards = stable_memory::get_latest_node_providers_rewards();

        if latest_np_rewards.timestamp < monthly_rewards.timestamp {
            stable_memory::insert_node_provider_rewards(monthly_rewards.timestamp, monthly_rewards)
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use candid::Principal;
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

    #[test]
    fn test_rewards_percent() {
        // Overall failed = 130 Overall total = 500 Failure rate = 0.26
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(20, 6, 4), MockedMetrics::new(25, 10, 2)]);
        let result = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result.rewards_multiplier, 0.744);

        // Overall failed = 45 Overall total = 450 Failure rate = 0.1
        // rewards_reduction = 0.0
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 400, 20),
            MockedMetrics::new(1, 5, 25), // no penalty
        ]);
        let result = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result.rewards_multiplier, 1.0);

        // Overall failed = 5 Overall total = 10 Failure rate = 0.5
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5), // no penalty
        ]);
        let result = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result.rewards_multiplier, 0.36);
    }

    #[test]
    fn test_rewards_percent_max_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 5, 95), // max failure rate
        ]);
        let result = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result.rewards_multiplier, 0.2);
    }

    #[test]
    fn test_rewards_percent_min_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 9, 1), // min failure rate
        ]);
        let result = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        assert_eq!(result.rewards_multiplier, 1.0);
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
            compute_rewards_multiplier(&daily_metrics_mid_gap, daily_metrics_mid_gap.len() as u64).rewards_multiplier,
            0.7866666666666666
        );

        assert_eq!(
            compute_rewards_multiplier(&daily_metrics_mid_gap, daily_metrics_mid_gap.len() as u64).rewards_multiplier,
            compute_rewards_multiplier(&daily_metrics_left_gap, daily_metrics_left_gap.len() as u64).rewards_multiplier
        );
        assert_eq!(
            compute_rewards_multiplier(&daily_metrics_right_gap, daily_metrics_right_gap.len() as u64).rewards_multiplier,
            compute_rewards_multiplier(&daily_metrics_left_gap, daily_metrics_left_gap.len() as u64).rewards_multiplier
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
        let result = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);
        daily_metrics.reverse();
        let result_rev = compute_rewards_multiplier(&daily_metrics, daily_metrics.len() as u64);

        assert_eq!(result.rewards_multiplier, 1.0);
        assert_eq!(result_rev.rewards_multiplier, result.rewards_multiplier);
    }
}
