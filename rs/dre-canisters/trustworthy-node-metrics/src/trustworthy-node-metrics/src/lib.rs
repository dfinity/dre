use candid::Principal;
use ic_cdk_macros::*;
use itertools::Itertools;
use std::{
    collections::{self, btree_map::Entry, BTreeMap},
    time::Duration,
};
use types::{
    DailyNodeMetrics, NodeMetrics, NodeMetricsStoredKey, NodeRewardsArgs, NodeRewardsResponse, SubnetNodeMetricsArgs,
    SubnetNodeMetricsResponse
};
mod metrics_manager;
mod rewards_manager;
mod stable_memory;
pub mod types;

// Management canisters updates node metrics every day
const TIMER_INTERVAL_SEC: u64 = 60 * 60 * 24;

async fn update_metrics_task() {
    match metrics_manager::update_metrics().await {
        Ok(_) => {
            ic_cdk::println!("Successfully updated metrics");
        }
        Err(e) => {
            ic_cdk::println!("Error updating metrics: {}", e);
        }
    }
}

fn setup_timers() {
    ic_cdk_timers::set_timer(Duration::from_secs(0), || ic_cdk::spawn(update_metrics_task()));
    ic_cdk_timers::set_timer_interval(Duration::from_secs(TIMER_INTERVAL_SEC), || ic_cdk::spawn(update_metrics_task()));
}

#[init]
fn init() {
    setup_timers();
}

#[post_upgrade]
fn post_upgrade() {
    setup_timers();
}

#[query]
fn subnet_node_metrics(args: SubnetNodeMetricsArgs) -> Result<Vec<SubnetNodeMetricsResponse>, String> {
    let from_ts = args.ts.unwrap_or_default();
    let mut subnet_node_metrics: BTreeMap<(u64, Principal), Vec<NodeMetrics>> = BTreeMap::new();

    let node_metrics: Vec<(NodeMetricsStoredKey, types::NodeMetricsStored)> = stable_memory::get_metrics_range(from_ts, None);

    for ((ts, node_id), node_metrics_value) in node_metrics {
        if let Some(subnet_id) = args.subnet_id {
            if subnet_id != node_metrics_value.subnet_assigned {
                continue;
            }
        }

        let result_key = (ts, node_metrics_value.subnet_assigned);
        let result_value: NodeMetrics = NodeMetrics {
            node_id,
            num_blocks_proposed_total: node_metrics_value.num_blocks_proposed_total,
            num_blocks_failures_total: node_metrics_value.num_blocks_failures_total,
        };

        match subnet_node_metrics.entry(result_key) {
            Entry::Occupied(mut entry) => {
                let v: &mut Vec<NodeMetrics> = entry.get_mut();
                v.push(result_value)
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![result_value]);
            }
        }
    }

    let result = subnet_node_metrics
        .into_iter()
        .map(|((ts, subnet_id), node_metrics)| SubnetNodeMetricsResponse { ts, subnet_id, node_metrics })
        .collect_vec();

    Ok(result)
}

#[query]
fn node_rewards(args: NodeRewardsArgs) -> Vec<NodeRewardsResponse> {
    let period_start = args.from_ts;
    let period_end = args.to_ts;
    let node_metrics: Vec<(NodeMetricsStoredKey, types::NodeMetricsStored)> = stable_memory::get_metrics_range(period_start, Some(period_end));

    let mut daily_metrics = collections::BTreeMap::new();
    for ((ts, node_id), node_metrics_value) in node_metrics {
        let daily_node_metrics = DailyNodeMetrics::new(
            ts,
            node_metrics_value.subnet_assigned,
            node_metrics_value.num_blocks_proposed,
            node_metrics_value.num_blocks_failed,
        );

        match daily_metrics.entry(node_id) {
            Entry::Occupied(mut entry) => {
                let v: &mut Vec<DailyNodeMetrics> = entry.get_mut();
                v.push(daily_node_metrics)
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![daily_node_metrics]);
            }
        }
    }

    daily_metrics
        .into_iter()
        .map(|(node_id, daily_node_metrics)| {
            let rewards_no_penalty = rewards_manager::rewards_no_penalty(&daily_node_metrics);
            let rewards_with_penalty = rewards_manager::rewards_with_penalty(&daily_node_metrics);

            NodeRewardsResponse {
                node_id,
                rewards_no_penalty,
                rewards_with_penalty,
                daily_node_metrics,
            }
        })
        .collect_vec()
}
