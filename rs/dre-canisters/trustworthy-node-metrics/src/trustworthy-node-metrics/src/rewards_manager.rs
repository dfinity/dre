use itertools::Itertools;
use num_traits::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trustworthy_node_metrics_types::types::{DailyNodeMetrics, RewardsComputationResult};

use crate::computation_logger::{ComputationLogger, Operation, OperationExecuted};

const MIN_FAILURE_RATE: u64 = 10;
const MAX_FAILURE_RATE: u64 = 70;

/// Calculates the rewards reduction based on the failure rate.
///
/// # Arguments
///
/// * `failure_rate` - A reference to a `u64` value representing the failure rate.
///
/// # Returns
///
/// * A `u64` value representing the rewards reduction, where:
///   - `0` indicates no reduction (failure rate below the minimum threshold),
///   - `100` indicates maximum reduction (failure rate above the maximum threshold),
///   - A value between `0` and `100` represents a proportional reduction based on the failure rate.
///
/// # Explanation
///
/// 1. The function checks if the provided `failure_rate` is below the `MIN_FAILURE_RATE` -> no reduction in rewards.
///
/// 2. It then checks if the `failure_rate` is above the `MAX_FAILURE_RATE` -> maximum reduction in rewards.
///
/// 3. If the `failure_rate` is within the defined range (`MIN_FAILURE_RATE` to `MAX_FAILURE_RATE`),
///    the function calculates the reduction proportionally:
///    - The reduction is calculated by normalizing the `failure_rate` within the range, resulting in a value between `0` and `100`.
fn rewards_reduction_percent(failure_rate: &u64) -> (Vec<OperationExecuted<u64>>, u64) {
    if failure_rate < &MIN_FAILURE_RATE {
        let (operation, result) = OperationExecuted::execute(
            &format!(
                "No Reduction applied because {}% is less than {}% failure rate",
                *failure_rate, MIN_FAILURE_RATE
            ),
            Operation::Set(0),
        );
        (vec![operation], result)
    } else if failure_rate > &MAX_FAILURE_RATE {
        let (operation, result) = OperationExecuted::execute(
            &format!(
                "Max reduction applied because {}% is over {}% failure rate",
                *failure_rate, MAX_FAILURE_RATE
            ),
            Operation::Set(100),
        );

        (vec![operation], result)
    } else {
        let (y_change_operation, y_change) =
            OperationExecuted::execute("Linear Reduction Y change", Operation::Subtract(*failure_rate, MIN_FAILURE_RATE));
        let (x_change_operation, x_change) =
            OperationExecuted::execute("Linear Reduction X change", Operation::Subtract(MAX_FAILURE_RATE, MIN_FAILURE_RATE));

        let (operation, result) = OperationExecuted::execute("Linear Reduction Percent", Operation::Percent(y_change, x_change));
        (vec![y_change_operation, x_change_operation, operation], result)
    }
}

/// Computes the rewards percentage based on the daily reward reductions WITH consecutive days penalty.
///
/// # Arguments
///
/// * `daily_metrics` - A slice of `DailyNodeMetrics` structs, where each struct represents the metrics for a single day.
///
/// # Returns
///
/// * A `f64` value representing the rewards percentage after reductions and penalties, rounded to two decimal places.
///
/// # Explanation
///
/// 1. The function calculates the number of active days.
///
/// 2. The function iterates over each day's metrics:
///    - For each day with a non-zero reduction, it adds the reduction to the cumulative penalty and increments the consecutive day penalty counter.
///    - For each day with zero reduction (`daily_reduction == 0.0`), if there was a streak of consecutive days with reductions,
///      it adds the cumulative penalty to the `reduction_sum` and resets the streak counters.
///
/// 3. The overall reduction is calculated by dividing the total reduction (`reduction_sum`) by the number of active days.
///    The reduction is then normalized by ensuring it does not exceed 1.0.
#[allow(dead_code)]
pub fn rewards_with_penalty(daily_metrics: &[DailyNodeMetrics]) -> f64 {
    let active_days = daily_metrics.len();
    let mut reduction_sum = 0.0;
    let mut consecutive_reduction = 0.0;
    let mut consecutive_count = 0;

    for metrics in daily_metrics.iter() {
        // Just if we want to count the day unassigned as 0.0 reduction
        // we would need to check if previous daily metrics is <= 24hrs
        // before current metrics
        let daily_reduction: f64 = metrics.num_blocks_failed as f64 / (metrics.num_blocks_failed + metrics.num_blocks_proposed) as f64;

        if daily_reduction == 0.0 {
            if consecutive_count > 0 {
                reduction_sum += consecutive_reduction * consecutive_count as f64;
                consecutive_reduction = 0.0;
                consecutive_count = 0;
            }
        } else {
            consecutive_reduction += daily_reduction;
            consecutive_count += 1;
        }
    }

    // Handles the last consecutive days
    if consecutive_count > 0 {
        reduction_sum += consecutive_reduction * consecutive_count as f64;
    }

    let overall_reduction = reduction_sum / active_days as f64;
    let reduction_normalized = overall_reduction.min(1.0);

    ((1.0 - reduction_normalized) * 100.0).round() / 100.0
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
/// * A `f64` value representing the rewards percentage left after the rewards reduction, rounded to two decimal places.
///
/// # Explanation
///
/// 1. The function iterates through each day's metrics, summing up the `daily_failed` and `daily_total` blocks across all days.
/// 2. The `overall_failure_rate` is calculated by dividing the `overall_failed` blocks by the `overall_total` blocks.
/// 3. The `rewards_reduction` function is applied to `overall_failure_rate`.
/// 3. Finally, the rewards percentage to be distrubuted to the node is computed.
pub fn compute_rewards_percent(daily_metrics: &[DailyNodeMetrics]) -> RewardsComputationResult {
    let mut computation_logger = ComputationLogger::new().with_input(computation_input);

    let daily_failed = daily_metrics.iter().map(|metrics| metrics.num_blocks_failed).collect_vec();
    let daily_proposed = daily_metrics.iter().map(|metrics| metrics.num_blocks_proposed).collect_vec();

    let overall_failed = computation_logger.execute("Computing Total Failed Blocks", Operation::Sum(daily_failed));
    let overall_proposed = computation_logger.execute("Computing Total Proposed Blocks", Operation::Sum(daily_proposed));
    let overall_total = computation_logger.execute("Computing Total Blocks", Operation::Sum(vec![overall_failed, overall_proposed]));
    let overall_failure_rate = computation_logger.execute("Computing Total Failure Rate", Operation::Percent(overall_failed, overall_total));
    let (operations, rewards_reduction) = rewards_reduction_percent(&overall_failure_rate);
    computation_logger.add_executed(operations);
    let rewards_percent = computation_logger.execute("Total Rewards Percent", Operation::Subtract(100, rewards_reduction));

    let computation_input = daily_metrics
        .iter()
        .map(|metric| metric.to_string()) // Convert each DailyNodeMetrics to string
        .collect_vec()
        .join("\n");

    let computation_logger = computation_logger.with_input(computation_input);

    RewardsComputationResult {
        rewards_percent: (Decimal::from(rewards_percent) / dec!(100)).to_f64().unwrap(),
        rewards_reduction: (Decimal::from(rewards_reduction) / dec!(100)).to_f64().unwrap(),
        blocks_failed: overall_failed,
        blocks_proposed: overall_proposed,
        blocks_total: overall_total,
        failure_rate: (Decimal::from(overall_failure_rate) / dec!(100)).to_f64().unwrap(),
        computation_log: computation_logger.get_log(),
    }
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
        // rewards_reduction = 0.266
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(20, 6, 4), MockedMetrics::new(25, 10, 2)]);
        let result = compute_rewards_percent(&daily_metrics);
        assert_eq!(result.rewards_percent, 0.73);

        // Overall failed = 45 Overall total = 450 Failure rate = 0.1
        // rewards_reduction = 0.0
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 400, 20),
            MockedMetrics::new(1, 5, 25), // no penalty
        ]);
        let result = compute_rewards_percent(&daily_metrics);
        assert_eq!(result.rewards_percent, 1.0);

        // Overall failed = 5 Overall total = 10 Failure rate = 0.5
        // rewards_reduction = 0.666
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5), // no penalty
        ]);
        let result = compute_rewards_percent(&daily_metrics);
        assert_eq!(result.rewards_percent, 0.33);
    }

    #[test]
    fn test_rewards_percent_max_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 5, 95), // max failure rate
        ]);
        let result = compute_rewards_percent(&daily_metrics);
        assert_eq!(result.rewards_percent, 0.0);
    }

    #[test]
    fn test_rewards_percent_min_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 9, 1), // min failure rate
        ]);
        let result = compute_rewards_percent(&daily_metrics);
        assert_eq!(result.rewards_percent, 1.0);
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

        assert_eq!(compute_rewards_percent(&daily_metrics_mid_gap).rewards_percent, 0.78);

        assert_eq!(
            compute_rewards_percent(&daily_metrics_mid_gap).rewards_percent,
            compute_rewards_percent(&daily_metrics_left_gap).rewards_percent
        );
        assert_eq!(
            compute_rewards_percent(&daily_metrics_right_gap).rewards_percent,
            compute_rewards_percent(&daily_metrics_left_gap).rewards_percent
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
        let result = compute_rewards_percent(&daily_metrics);
        daily_metrics.reverse();
        let result_rev = compute_rewards_percent(&daily_metrics);

        assert_eq!(result.rewards_percent, 1.0);
        assert_eq!(result_rev.rewards_percent, result.rewards_percent);
    }
}
