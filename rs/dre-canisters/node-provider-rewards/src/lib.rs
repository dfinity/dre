use candid::Principal;
use chrono_utils::DateTimeRange;
use ic_cdk_macros::*;
use itertools::Itertools;
use rewards_manager::RewardsManager;
use std::collections::{btree_map::Entry, BTreeMap};
use types::{NodeMetrics, NodeMetricsStored, NodeMetricsStoredKey, SubnetNodeMetricsArgs, SubnetNodeMetricsResponse};
mod chrono_utils;
mod computation_logger;
mod nodes_manager;
mod rewards_manager;
mod stable_memory;
mod types;

// Management canisters updates node metrics every day
const TIMER_INTERVAL_SEC: u64 = 60 * 60 * 24;

async fn sync_node_metrics_task() {
    match nodes_manager::sync_node_metrics().await {
        Ok(_) => {
            ic_cdk::println!("Successfully updated trustworthy node metrics");
        }
        Err(e) => {
            ic_cdk::println!("Error syncing metrics: {}", e);
        }
    }
}

async fn update_rewardable_nodes_task() {
    let mut rewards_manager = RewardsManager::new();
    let now = ic_cdk::api::time();
    let rewarding_period: DateTimeRange = DateTimeRange::last_reward_period(now);

    ic_cdk::println!("Computing and storing rewards for {}", rewarding_period);
    let node_operators_rewardables = nodes_manager::get_node_operators_rewardables(&rewarding_period).await.unwrap();

    let rewards_table = nodes_manager::get_rewards_table().await.unwrap();
    let assigned_nodes_performance = nodes_manager::get_assigned_nodes_performance(node_operators_rewardables.clone(), &rewarding_period).await.unwrap();
    let node_providers_rewardables = nodes_manager::get_node_providers_rewardables(node_operators_rewardables);

    rewards_manager.compute_node_providers_rewards(node_providers_rewardables, assigned_nodes_performance, rewarding_period, rewards_table);
}

fn setup_timers() {

    ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(TIMER_INTERVAL_SEC), || ic_cdk::spawn(sync_node_metrics_task()));
    
    ic_cdk_timers::set_timer(std::time::Duration::from_secs(5), || ic_cdk::spawn(update_rewardable_nodes_task()));
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

    let node_metrics: Vec<(NodeMetricsStoredKey, NodeMetricsStored)> = stable_memory::get_metrics_range(from_ts, None, None);

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
fn node_provider_(args: SubnetNodeMetricsArgs) -> Result<Vec<SubnetNodeMetricsResponse>, String> {
    let from_ts = args.ts.unwrap_or_default();
    let mut subnet_node_metrics: BTreeMap<(u64, Principal), Vec<NodeMetrics>> = BTreeMap::new();

    let node_metrics: Vec<(NodeMetricsStoredKey, NodeMetricsStored)> = stable_memory::get_metrics_range(from_ts, None, None);

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

