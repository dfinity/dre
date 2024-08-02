use itertools::Itertools;

use crate::types::{NodeMetrics, Rewards, TimestampNanos};

#[derive(Debug)]
pub struct DailyMetrics {
    pub proposed_blocks: u64,
    pub failed_blocks: u64,
}

impl DailyMetrics {
    pub fn new(proposed_blocks: u64, failed_blocks: u64) -> Self {
        DailyMetrics {
            proposed_blocks,
            failed_blocks,
        }
    }

    pub fn failure_rates(&self) -> f64 {
        let total_blocks = self.failed_blocks + self.proposed_blocks;
        if total_blocks == 0 {
            0.0
        } else {
            self.failed_blocks as f64 / total_blocks as f64
        }
    }
}

const MIN_FAILURE_RATE: f64 = 0.1;
const MAX_FAILURE_RATE: f64 = 0.7;

fn calculate_rewards_no_penalty(failure_rates: &[f64]) -> f64 {
    let sum: f64 = failure_rates.iter().sum();
    let average_failure_rate = sum / failure_rates.len() as f64;

    let reduction = if average_failure_rate < MIN_FAILURE_RATE {
        0.0
    } else if average_failure_rate > MAX_FAILURE_RATE {
        1.0
    } else {
        (average_failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)
    };

    (1.0 - reduction) * 100.0
}

fn calculate_rewards_with_penalty(failure_rates: &[f64]) -> f64 {
    let mut overall_reduction = 0.0;
    let mut consecutive_days = 0;
    let mut daily_reductions = Vec::new();
    let assigned_days = failure_rates.len() as f64;

    for &daily_failure_rate in failure_rates {
        daily_reductions.push(daily_failure_rate);

        if daily_failure_rate >= MIN_FAILURE_RATE {
            consecutive_days += 1;
        } else if consecutive_days > 0 {
            let penalty_weight = (consecutive_days * consecutive_days) as f64 / assigned_days;
            let sum: f64 = failure_rates
                .iter()
                .skip(daily_reductions.len() - consecutive_days)
                .take(consecutive_days)
                .sum();
            let average_failure_rate = sum / consecutive_days as f64;
            let penalty_reduction = average_failure_rate * penalty_weight;
            overall_reduction += penalty_reduction;
            consecutive_days = 0;
        }
    }

    if consecutive_days > 0 {
        let penalty_weight = (consecutive_days * consecutive_days) as f64 / 30.0;
        let sum: f64 = failure_rates
            .iter()
            .skip(daily_reductions.len() - consecutive_days)
            .take(consecutive_days)
            .sum();
        let average_failure_rate = sum / consecutive_days as f64;
        let penalty_reduction = average_failure_rate * penalty_weight;
        overall_reduction += penalty_reduction;
    }

    overall_reduction += daily_reductions.iter().sum::<f64>();

    100.0 - overall_reduction
}

pub fn compute_rewards(mut metrics: Vec<(TimestampNanos, NodeMetrics)>, initial_metrics: NodeMetrics) -> Rewards {
    let mut daily_metrics = Vec::new();

    metrics.sort_by_key(|&(timestamp, _)| timestamp);

    let mut previous_failed_total = initial_metrics.num_block_failures_total;
    let mut previous_proposed_total = initial_metrics.num_blocks_proposed_total;

    for (_, node_metrics) in metrics {
        let daily_proposed = node_metrics.num_blocks_proposed_total - previous_proposed_total;
        let daily_failed = node_metrics.num_block_failures_total - previous_failed_total;

        daily_metrics.push(DailyMetrics::new(daily_proposed, daily_failed));

        previous_failed_total = node_metrics.num_block_failures_total;
        previous_proposed_total = node_metrics.num_blocks_proposed_total;
    }

    let failure_rates = daily_metrics.iter().map(|metrics| metrics.failure_rates()).collect_vec();

    Rewards {
        rewards_standard: calculate_rewards_no_penalty(&failure_rates),
        rewards_with_penalty: calculate_rewards_with_penalty(&failure_rates),
    }
}
