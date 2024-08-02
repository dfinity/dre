use candid::Principal;
use ic_cdk_macros::*;
use itertools::Itertools;
use std::{collections, time::Duration};
use types::{
     NodeMetrics, NodeRewardsArgs, NodeRewardsResponse, SubnetNodeMetricsArgs,
    SubnetNodeMetricsResponse, TimestampNanos,
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

    let metrics = stable_memory::get_metrics_range(from_ts, None);

    let metrics_flat = metrics
        .into_iter()
        .flat_map(|(ts, subnets)| {
            subnets.into_iter().map(move |subnet_node_metrics| SubnetNodeMetricsResponse {
                ts,
                subnet_id: subnet_node_metrics.subnet_id,
                node_metrics: subnet_node_metrics.node_metrics,
            })
        })
        .collect_vec();

    let result = match args.subnet_id {
        Some(subnet_id) => metrics_flat.into_iter().filter(|metrics| metrics.subnet_id == subnet_id).collect_vec(),
        None => metrics_flat,
    };

    Ok(result)
}

#[query]
fn node_rewards(args: NodeRewardsArgs) -> Vec<NodeRewardsResponse> {
    let period_start = args.from_ts;
    let period_end = args.to_ts;
    let metrics = stable_memory::get_metrics_range(period_start, Some(period_end));

    let mut metrics_by_node: collections::BTreeMap<Principal, Vec<(TimestampNanos, NodeMetrics)>> = collections::BTreeMap::new();
    for (ts, subnets) in metrics {
        for subnet_metrics in subnets {
            for node_metrics in subnet_metrics.node_metrics {
                metrics_by_node.entry(node_metrics.node_id).or_default().push((ts, node_metrics));
            }
        }
    }

    let node_ids = metrics_by_node.keys().cloned().collect_vec();
    let initial_node_metrics = stable_memory::metrics_before_ts(node_ids, &period_start);


    let result = metrics_by_node
        .into_iter()
        .map(|(node_id, metrics_in_period)| {
            let node_rewards = rewards_manager::compute_rewards(metrics_in_period, initial_node_metrics.get(&node_id).cloned().unwrap());

            NodeRewardsResponse { node_id, node_rewards }
        })
        .collect_vec();

    result
}
