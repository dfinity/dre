use candid::Principal;

use crate::types::{DailyMetrics, NodeMetrics, TimestampNanos};

const MIN_FAILURE_RATE: f64 = 0.1;
const MAX_FAILURE_RATE: f64 = 0.7;

fn daily_rewards_reduction(failure_rate: &f64) -> f64 {
    if failure_rate < &MIN_FAILURE_RATE {
        0.0
    } else if failure_rate > &MAX_FAILURE_RATE {
        1.0
    } else {
        let reduction = (failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE);
        (reduction * 100.0).round() / 100.0 
    }
}

impl DailyMetrics {
    pub fn new(ts: TimestampNanos, subnet_assignment: Principal, proposed_blocks: u64, failed_blocks: u64) -> Self {
        let total_blocks = failed_blocks + proposed_blocks;
        let failure_rate = if total_blocks == 0 {
            0.0
        } else {
            failed_blocks as f64 / total_blocks as f64
        };
        
        let rewards_reduction = daily_rewards_reduction(&failure_rate);

        DailyMetrics {
            ts,
            subnet_assigned: subnet_assignment,
            failure_rate,
            rewards_reduction,
        }
    }
}

pub fn rewards_with_penalty(daily_metrics: &[DailyMetrics]) -> f64 {
    let active_days = daily_metrics.len();
    let mut reduction = 0.0;
    let mut consecutive_reduction = 0.0;
    let mut consecutive_count = 0;

    for metrics in daily_metrics.iter() {
        // Just if we want to count the day unassigned as 0.0 reduction
        // we would need to check if previous daily metrics is <= 24hrs
        // before current metrics
        let daily_rewards: f64 = metrics.rewards_reduction;
    
        if daily_rewards == 0.0 {
            if consecutive_count > 0 {
                reduction += consecutive_reduction * consecutive_count as f64;
                consecutive_reduction = 0.0;
                consecutive_count = 0;
            }
        } else {
            consecutive_reduction += daily_rewards;
            consecutive_count += 1; 
        }
    }
    
    // Handles the last consecutive days
    if consecutive_count > 0 {
        reduction += consecutive_reduction * consecutive_count as f64;
    }

    reduction /= active_days as f64;
    let reduction_normalized = reduction.min(1.0);
    
    ((1.0 - reduction_normalized) * 100.0).round() / 100.0
}

pub fn rewards_no_penalty(daily_metrics: &[DailyMetrics]) -> f64 {
    let active_days = daily_metrics.len();

    let reduction = daily_metrics.iter().fold(0.0, |mut acc, metrics| {
        let daily_rewards = metrics.rewards_reduction;

        acc += daily_rewards;
        acc
    }) / (active_days as f64);

    ((1.0 - reduction) * 100.0).round() / 100.0
}

pub fn daily_metrics(mut node_metrics: Vec<(TimestampNanos, NodeMetrics, Principal)>, initial_metrics: NodeMetrics) -> Vec<DailyMetrics> {
    let mut failure_rates = Vec::new();

    node_metrics.sort_by_key(|&(timestamp, _, _)| timestamp);

    let mut previous_failed_total = initial_metrics.num_block_failures_total;
    let mut previous_proposed_total = initial_metrics.num_blocks_proposed_total;

    for (ts_nanos, node_metrics, subnet_id) in node_metrics {
        let current_failed_total = node_metrics.num_block_failures_total;
        let current_proposed_total = node_metrics.num_blocks_proposed_total;

        if previous_failed_total > current_failed_total || previous_proposed_total > current_proposed_total {
            // This is the case when the machine gets redeployed
            previous_failed_total = 0;
            previous_proposed_total = 0;
        };

        let daily_failed = current_failed_total - previous_failed_total;
        let daily_proposed = current_proposed_total - previous_proposed_total;

        failure_rates.push(DailyMetrics::new(ts_nanos, subnet_id, daily_proposed, daily_failed));

        previous_failed_total = current_failed_total;
        previous_proposed_total = current_proposed_total;
    }

    failure_rates
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

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

    fn daily_mocked_metrics(metrics: Vec<MockedMetrics>) -> Vec<DailyMetrics> {
        let subnet = Principal::anonymous();
        let mut i = 0;

        metrics.into_iter().flat_map(|mocked_metrics: MockedMetrics|
            (0..mocked_metrics.days).map(move |_| {
                let next_ts = i * 24 * 60 * 60 * 1_000_000_000;
                i += 1;
                DailyMetrics::new(next_ts, subnet, mocked_metrics.proposed_blocks, mocked_metrics.failed_blocks)
            })
        ).collect_vec()
    }

    #[test]
    fn test_rewards_no_penalty() {
        let metrics: Vec<MockedMetrics> = vec![
            MockedMetrics::new(10, 6, 4)
        ];

        let daily_metrics = daily_mocked_metrics(metrics);
        let no_penalty = rewards_no_penalty(&daily_metrics);
        assert_eq!(no_penalty, 0.5);
    }

    #[test]
    fn test_rewards_with_penalty() {
        let metrics: Vec<MockedMetrics> = vec![
            MockedMetrics::new(5, 6, 4),
            MockedMetrics::new(25, 10, 0),
        ];

        let daily_metrics = daily_mocked_metrics(metrics);
        let with_penalty = rewards_with_penalty(&daily_metrics);
        assert_eq!(with_penalty, 0.58);
    }

    #[test]
    fn test_rewards_with_penalty_min_0() {
        let metrics: Vec<MockedMetrics> = vec![
            MockedMetrics::new(5, 6, 4)
        ];

        let daily_metrics = daily_mocked_metrics(metrics);
        let with_penalty = rewards_with_penalty(&daily_metrics);
        assert_eq!(with_penalty, 0.0);
    }

    #[test]
    fn test_rewards_with_penalty_2_days() {
        let metrics: Vec<MockedMetrics> = vec![
            MockedMetrics::new(5, 6, 4)
        ];

        let daily_metrics = daily_mocked_metrics(metrics);
        let with_penalty = rewards_with_penalty(&daily_metrics);
        assert_eq!(with_penalty, 0.0);
    }

}
