use chrono::{DateTime, Duration, Utc};

use crate::types::{NodeMetrics, Rewards, TimestampNanos};

#[derive(Debug)]
pub struct DailyFailureRate {
    pub datetime: DateTime<Utc>,
    pub failure_rate: f64,
}

impl DailyFailureRate {
    pub fn new(ts_nanos: TimestampNanos, proposed_blocks: u64, failed_blocks: u64) -> Self {
        let secs = (ts_nanos / 1_000_000_000) as i64;
        let nanos = (secs % 1_000_000_000) as u32;
        let total_blocks = failed_blocks + proposed_blocks;
        let failure_rate = if total_blocks == 0 {
            0.0
        } else {
            failed_blocks as f64 / total_blocks as f64
        };

        DailyFailureRate {
            datetime: chrono::DateTime::<Utc>::from_timestamp(secs, nanos).unwrap(),
            failure_rate,
        }
    }
}

const MIN_FAILURE_RATE: f64 = 0.1;
const MAX_FAILURE_RATE: f64 = 0.7;

fn daily_reduction(failure_rate: &f64) -> f64 {
    if failure_rate < &MIN_FAILURE_RATE {
        0.0
    } else if failure_rate > &MAX_FAILURE_RATE {
        1.0
    } else {
        (failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)
    }
}

fn calculate_reduction(metrics: &[DailyFailureRate]) -> (f64, f64) {
    let active_days = metrics.len();

    let mut day_tracker = metrics[0].datetime;
    let mut consecutive_days = Vec::new();
    let mut reduction_with_penalty = 0 as f64;

    let reduction_no_penalty = metrics.iter().fold(0 as f64, |mut acc, daily_metrics| {
        acc += daily_reduction(&daily_metrics.failure_rate);
        acc
    }) / (active_days as f64);

    for daily_metrics in metrics {
        let daily_reduction = daily_reduction(&daily_metrics.failure_rate);
        let is_day_after = daily_metrics.datetime.date_naive() == day_tracker.date_naive() + Duration::days(1);

        if daily_reduction == 0 as f64 {
            if !consecutive_days.is_empty() {
                reduction_with_penalty += consecutive_days.len() as f64 * consecutive_days.iter().sum::<f64>();
                consecutive_days.clear();
            }
        } else if is_day_after {
            consecutive_days.push(daily_reduction)
        } else {
            reduction_with_penalty += daily_reduction
        }

        day_tracker = daily_metrics.datetime;
    }

    reduction_with_penalty /= active_days as f64;

    (reduction_no_penalty, reduction_with_penalty)
}

pub fn compute_rewards(mut metrics: Vec<(TimestampNanos, NodeMetrics)>, initial_metrics: NodeMetrics) -> Rewards {
    let mut daily_failure_rate = Vec::new();

    metrics.sort_by_key(|&(timestamp, _)| timestamp);

    let mut previous_failed_total = initial_metrics.num_block_failures_total;
    let mut previous_proposed_total = initial_metrics.num_blocks_proposed_total;

    for (ts_nanos, node_metrics) in metrics {
        let daily_proposed = node_metrics.num_blocks_proposed_total - previous_proposed_total;
        let daily_failed = node_metrics.num_block_failures_total - previous_failed_total;

        daily_failure_rate.push(DailyFailureRate::new(ts_nanos, daily_proposed, daily_failed));

        previous_failed_total = node_metrics.num_block_failures_total;
        previous_proposed_total = node_metrics.num_blocks_proposed_total;
    }

    let (reduction_no_penalty, reduction_with_penalty) = calculate_reduction(&daily_failure_rate);

    Rewards {
        rewards_standard: (1.0 - reduction_no_penalty) * 100.0,
        rewards_with_penalty: (1.0 - reduction_with_penalty) * 100.0,
    }
}
