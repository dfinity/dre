use candid::Principal;
use chrono::{Duration, Utc};

use crate::types::{DailyNodeData, NodeMetrics, TimestampNanos};

impl DailyNodeData {
    pub fn new(ts: TimestampNanos, subnet_id: Principal, proposed_blocks: u64, failed_blocks: u64) -> Self {
        let total_blocks = failed_blocks + proposed_blocks;
        let failure_rate = if total_blocks == 0 {
            0.0
        } else {
            failed_blocks as f64 / total_blocks as f64
        };

        DailyNodeData { ts, subnet_id, failure_rate }
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

fn is_one_day_after(ts1: u64, ts2: u64) -> bool {
    let ts1_sec = ts1 / 1_000_000_000;
    let ts2_sec = ts2 / 1_000_000_000;

    let dt1 = chrono::DateTime::<Utc>::from_timestamp(ts1_sec as i64, 0).unwrap();
    let dt2 = chrono::DateTime::<Utc>::from_timestamp(ts2_sec as i64, 0).unwrap();

    dt2.date_naive() == dt1.date_naive() - Duration::days(1)
}

pub fn rewards_with_penalty(daily_data: &[DailyNodeData]) -> f64 {
    let active_days = daily_data.len();

    let mut previous_ts = daily_data[0].ts;
    let mut consecutive_days = Vec::new();
    let mut reduction_with_penalty = 0 as f64;

    for node_data in daily_data {
        let daily_reduction = daily_reduction(&node_data.failure_rate);
        let is_day_after = is_one_day_after(node_data.ts, previous_ts);

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

        previous_ts = node_data.ts
    }

    reduction_with_penalty /= active_days as f64;

    1.0 - reduction_with_penalty
}

pub fn rewards_no_penalty(daily_data: &[DailyNodeData]) -> f64 {
    let active_days = daily_data.len();

    let reduction_no_penalty = daily_data.iter().fold(0 as f64, |mut acc, data| {
        acc += daily_reduction(&data.failure_rate);
        acc
    }) / (active_days as f64);

    1.0 - reduction_no_penalty
}

pub fn daily_data(mut metrics: Vec<(TimestampNanos, NodeMetrics, Principal)>, initial_metrics: NodeMetrics) -> Vec<DailyNodeData> {
    let mut failure_rates = Vec::new();

    metrics.sort_by_key(|&(timestamp, _, _)| timestamp);

    let mut previous_failed_total = initial_metrics.num_block_failures_total;
    let mut previous_proposed_total = initial_metrics.num_blocks_proposed_total;

    for (ts_nanos, node_metrics, subnet_id) in metrics {
        let current_failed_total = node_metrics.num_block_failures_total;
        let current_proposed_total = node_metrics.num_blocks_proposed_total;

        if previous_failed_total > current_failed_total || previous_proposed_total > current_proposed_total {
            // This is the case when the machine gets redeployed
            previous_failed_total = 0;
            previous_proposed_total = 0;
        };

        let daily_failed = current_failed_total - previous_failed_total;
        let daily_proposed = current_proposed_total - previous_proposed_total;

        failure_rates.push(DailyNodeData::new(ts_nanos, subnet_id, daily_proposed, daily_failed));

        previous_failed_total = current_failed_total;
        previous_proposed_total = current_proposed_total;
    }

    failure_rates
}
