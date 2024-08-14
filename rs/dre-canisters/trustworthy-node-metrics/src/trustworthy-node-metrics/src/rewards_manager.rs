use candid::Principal;

use crate::types::{DailyNodeMetrics, TimestampNanos};

const MIN_FAILURE_RATE: f64 = 0.1;
const MAX_FAILURE_RATE: f64 = 0.7;

/// Calculates the daily rewards reduction based on the failure rate.
///
/// # Arguments
///
/// * `failure_rate` - A reference to a `f64` value representing the failure rate for the day.
///
/// # Returns
///
/// * A `f64` value representing the rewards reduction for the day, where:
///   - `0.0` indicates no reduction (failure rate below the minimum threshold),
///   - `1.0` indicates maximum reduction (failure rate above the maximum threshold),
///   - A value between `0.0` and `1.0` represents a proportional reduction based on the failure rate.
///
/// # Explanation
///
/// 1. The function checks if the provided `failure_rate` is below the `MIN_FAILURE_RATE` -> no reduction in rewards.
///
/// 2. It then checks if the `failure_rate` is above the `MAX_FAILURE_RATE` -> maximum reduction in rewards.
///
/// 3. If the `failure_rate` is within the defined range (`MIN_FAILURE_RATE` to `MAX_FAILURE_RATE`),
///    the function calculates the reduction proportionally:
///    - The reduction is calculated by normalizing the `failure_rate` within the range, resulting in a value between `0.0` and `1.0`.
fn daily_rewards_reduction(failure_rate: &f64) -> f64 {
    if failure_rate < &MIN_FAILURE_RATE {
        0.0
    } else if failure_rate > &MAX_FAILURE_RATE {
        1.0
    } else {
        (failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)
    }
}

impl DailyNodeMetrics {
    pub fn new(ts: TimestampNanos, subnet_assignment: Principal, proposed_blocks: u64, failed_blocks: u64) -> Self {
        let total_blocks = failed_blocks + proposed_blocks;
        let failure_rate = if total_blocks == 0 {
            0.0
        } else {
            failed_blocks as f64 / total_blocks as f64
        };

        let rewards_reduction = daily_rewards_reduction(&failure_rate);

        DailyNodeMetrics {
            ts,
            subnet_assigned: subnet_assignment,
            num_blocks_proposed: proposed_blocks,
            num_blocks_failed: failed_blocks,
            failure_rate,
            rewards_reduction,
        }
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
pub fn rewards_with_penalty(daily_metrics: &[DailyNodeMetrics]) -> f64 {
    let active_days = daily_metrics.len();
    let mut reduction_sum = 0.0;
    let mut consecutive_reduction = 0.0;
    let mut consecutive_count = 0;

    for metrics in daily_metrics.iter() {
        // Just if we want to count the day unassigned as 0.0 reduction
        // we would need to check if previous daily metrics is <= 24hrs
        // before current metrics
        let daily_reduction: f64 = metrics.rewards_reduction;

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

/// Rewards no penalty
///
/// Computes the rewards percentage based on the daily reward reductions WITHOUT consecutive days penalty.
/// `overall_reduction` is defined as the average of the daily reward reductions.
///
/// # Arguments
///
/// * `daily_metrics` - A slice of `DailyNodeMetrics` structs, where each struct represents the metrics for a single day.
///
/// # Returns
///
/// * A `f64` value representing the rewards percentage after overall reductions, rounded to two decimal places.
///
/// # Explanation
///
/// 1. The function iterates through each day's metrics, summing up the `rewards_reduction` values across all days.
/// 2. The `overall_reduction` is calculated by dividing the total `reduction_sum` by the number of active days.
/// 3. Finally, the function calculates the rewards percentage without penalty by subtracting `overall_reduction` from 1.0,
///    multiplying by 100, and rounding to two decimal places before returning the result.
pub fn rewards_no_penalty(daily_metrics: &[DailyNodeMetrics]) -> f64 {
    let active_days = daily_metrics.len();
    let reduction_sum: f64 = daily_metrics.iter().map(|metrics| metrics.rewards_reduction).sum();
    let overall_reduction = reduction_sum / (active_days as f64);

    ((1.0 - overall_reduction) * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
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
    fn test_rewards_no_penalty() {
        // Failure Rate = 40% Rewards reduction = 50%
        // 0.5 * 5 days / 30 days = 0.08 -> Rewards 0.92
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(5, 6, 4), MockedMetrics::new(25, 10, 0)]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 0.92);

        // Failure Rate = 40% Rewards reduction = 50%
        // 0.5 / 2 days = 0.25 -> Rewards 0.75
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 6, 4),
            MockedMetrics::new(1, 91, 9), // no penalty
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 0.75);

        // Failure Rate = 40% Rewards reduction = 50% 10 days
        // Failure Rate = 30% Rewards reduction = 33%
        // (0.5 * 10 + 0.33) / 11 = 0.48 Rewards 0.67
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 7, 3), // no penalty
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 0.67);
    }

    #[test]
    fn test_rewards_with_penalty() {
        // Failure Rate = 40% Rewards reduction = 50%
        // 0.5 * 5^2 days / 30 days = 0.42 -> Rewards 0.58
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(5, 6, 4), MockedMetrics::new(25, 10, 0)]);
        let rewards = rewards_with_penalty(&daily_metrics);
        assert_eq!(rewards, 0.58);

        // Failure Rate = 40% Rewards reduction = 50%
        // 0.5 / 2 days = 0.25 -> Rewards 0.75
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 6, 4),
            MockedMetrics::new(1, 91, 9), // no penalty
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 0.75);

        // Failure Rate = 40% Rewards reduction = 50% 10 days
        // Failure Rate = 30% Rewards reduction = 33%
        // (0.5 * 10 + 0.33) / 11 -> Reduction 0.48 Rewards 0.67
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 7, 3), // no penalty
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 0.67);
    }

    #[test]
    fn test_rewards_no_penalty_max_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 5, 95), // max failure rate
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 0.0);
    }

    #[test]
    fn test_rewards_with_penalty_max_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 9, 91), // max failure rate
        ]);
        let rewards = rewards_with_penalty(&daily_metrics);
        assert_eq!(rewards, 0.0);
    }

    #[test]
    fn test_rewards_no_penalty_min_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 9, 1), // min failure rate
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 1.0);
    }

    #[test]
    fn test_rewards_with_penalty_min_reduction() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(10, 9, 1), // min failure rate
        ]);
        let rewards = rewards_no_penalty(&daily_metrics);
        assert_eq!(rewards, 1.0);
    }

    #[test]
    fn test_same_rewards_if_gaps_no_penalty() {
        let gap = MockedMetrics::new(1, 10, 0);

        let daily_metrics_mid_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![MockedMetrics::new(1, 6, 4), gap.clone(), MockedMetrics::new(1, 7, 3)]);

        let daily_metrics_left_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![gap.clone(), MockedMetrics::new(1, 6, 4), MockedMetrics::new(1, 7, 3)]);

        let daily_metrics_right_gap: Vec<DailyNodeMetrics> =
            daily_mocked_metrics(vec![gap.clone(), MockedMetrics::new(1, 6, 4), MockedMetrics::new(1, 7, 3)]);

        assert_eq!(rewards_no_penalty(&daily_metrics_mid_gap), 0.72);

        assert_eq!(rewards_no_penalty(&daily_metrics_mid_gap), rewards_no_penalty(&daily_metrics_left_gap));
        assert_eq!(rewards_no_penalty(&daily_metrics_right_gap), rewards_no_penalty(&daily_metrics_left_gap));
    }

    #[test]
    fn test_less_rewards_if_consecutive_with_penalty() {
        let gap = MockedMetrics::new(1, 10, 0);

        let daily_metrics_no_gap: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(4, 8, 2)]);

        let daily_metrics_with_gap: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![MockedMetrics::new(2, 8, 2), gap, MockedMetrics::new(2, 8, 2)]);

        assert_eq!(rewards_with_penalty(&daily_metrics_no_gap), 0.33);

        assert!(rewards_with_penalty(&daily_metrics_with_gap) > rewards_with_penalty(&daily_metrics_no_gap));
    }

    #[test]
    fn test_same_rewards_if_reversed() {
        let daily_metrics: Vec<DailyNodeMetrics> = daily_mocked_metrics(vec![
            MockedMetrics::new(1, 5, 5),
            MockedMetrics::new(5, 6, 4),
            MockedMetrics::new(25, 10, 0),
        ]);

        let mut daily_metrics_1 = daily_metrics.clone();
        let rewards = rewards_with_penalty(&daily_metrics_1);
        daily_metrics_1.reverse();
        let rewards_rev = rewards_with_penalty(&daily_metrics_1);

        assert_eq!(rewards, 0.39);
        assert_eq!(rewards_rev, rewards);

        let mut daily_metrics_2 = daily_metrics.clone();
        let rewards = rewards_no_penalty(&daily_metrics_2);
        daily_metrics_2.reverse();
        let rewards_rev = rewards_no_penalty(&daily_metrics_2);

        assert_eq!(rewards, 0.9);
        assert_eq!(rewards_rev, rewards);
    }
}
