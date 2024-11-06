use std::collections::BTreeMap;
use candid::Principal;
use futures::FutureExt;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use ic_registry_keys::
    make_subnet_list_record_key
;
use ic_types::PrincipalId;

use crate::chrono_utils::DateTimeRange;
use crate::stable_memory::{self};
use crate::types::DailyNodeMetrics;
use crate::types::{NodeMetricsGrouped, NodeMetricsStored, NodeMetricsStoredKey};
use crate::types::{SubnetNodeMetricsHistory, TimestampNanos};
use itertools::Itertools;

/// Node metrics storable
///
/// Computes daily proposed/failed blocks from a vector of node metrics
fn node_metrics_storable(
    node_id: PrincipalId,
    node_metrics_grouped: Vec<NodeMetricsGrouped>,
    initial_proposed_total: u64,
    initial_failed_total: u64,
) -> Vec<(NodeMetricsStoredKey, NodeMetricsStored)> {
    let mut metrics_ordered = node_metrics_grouped;
    metrics_ordered.sort_by_key(|(ts, _, _)| *ts);

    let principal = node_id.0;
    let mut node_metrics_storable = Vec::new();

    let mut previous_proposed_total = initial_proposed_total;
    let mut previous_failed_total = initial_failed_total;

    for (ts, subnet_assigned, metrics) in metrics_ordered {
        let key = (ts, principal);
        let current_proposed_total = metrics.num_blocks_proposed_total;
        let current_failed_total = metrics.num_block_failures_total;

        let (daily_proposed, daily_failed) = calculate_daily_metrics(
            previous_proposed_total,
            previous_failed_total,
            metrics.num_blocks_proposed_total,
            metrics.num_block_failures_total,
        );

        let node_metrics_stored = NodeMetricsStored {
            subnet_assigned: subnet_assigned.0,
            num_blocks_proposed_total: current_proposed_total,
            num_blocks_failures_total: current_failed_total,
            num_blocks_proposed: daily_proposed,
            num_blocks_failed: daily_failed,
        };

        node_metrics_storable.push((key, node_metrics_stored));

        previous_proposed_total = current_proposed_total;
        previous_failed_total = current_failed_total;
    }

    node_metrics_storable
}

/// Fetch metrics
///
/// Calls to the node_metrics_history endpoint of the management canister for all the subnets
/// to get updated metrics since refresh_ts.
async fn fetch_metrics(subnets: Vec<PrincipalId>, refresh_ts: TimestampNanos) -> anyhow::Result<Vec<SubnetNodeMetricsHistory>> {
    let mut subnets_node_metrics = Vec::new();

    for subnet_id in subnets {
        let contract = NodeMetricsHistoryArgs {
            subnet_id,
            start_at_timestamp_nanos: refresh_ts,
        };

        let node_metrics = ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
            candid::Principal::management_canister(),
            "node_metrics_history",
            (contract,),
            0_u128,
        )
        .map(move |result| {
            result
                .map_err(|(code, msg)| {
                    anyhow::anyhow!(
                        "Error when calling management canister for subnet {}:\n Code:{:?}\nMsg:{}",
                        subnet_id,
                        code,
                        msg
                    )
                })
                .map(|(node_metrics,)| (subnet_id, node_metrics))
        });

        subnets_node_metrics.push(node_metrics);
    }

    let updated_metrics = futures::future::try_join_all(subnets_node_metrics).await?;

    for (subnet, node_metrics) in &updated_metrics {
        ic_cdk::println!("Fetched {} new metrics for subnet: {}", node_metrics.len(), subnet);
    }

    Ok(updated_metrics)
}

// Calculates the daily proposed and failed blocks
fn calculate_daily_metrics(last_proposed_total: u64, last_failed_total: u64, current_proposed_total: u64, current_failed_total: u64) -> (u64, u64) {
    if last_failed_total > current_failed_total || last_proposed_total > current_proposed_total {
        // This is the case when node gets redeploied
        (current_proposed_total, current_failed_total)
    } else {
        (current_proposed_total - last_proposed_total, current_failed_total - last_failed_total)
    }
}

fn grouped_by_node(subnet_metrics: Vec<(PrincipalId, Vec<NodeMetricsHistoryResponse>)>) -> BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>> {
    let mut grouped_by_node: BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>> = BTreeMap::new();

    for (subnet_id, history) in subnet_metrics {
        for history_response in history {
            for metrics in history_response.node_metrics {
                grouped_by_node
                    .entry(metrics.node_id)
                    .or_default()
                    .push((history_response.timestamp_nanos, subnet_id, metrics));
            }
        }
    }
    grouped_by_node
}

/// Fetch subnets
///
/// Fetch subnets from the registry canister
async fn fetch_subnets() -> anyhow::Result<Vec<PrincipalId>> {
    let (registry_subnets, _): (SubnetListRecord, _) = ic_nns_common::registry::get_value(make_subnet_list_record_key().as_bytes(), None).await?;
    let subnets = registry_subnets
        .subnets
        .into_iter()
        .map(|subnet_id: Vec<u8>| PrincipalId::try_from(subnet_id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(subnets)
}

/// Update node metrics
pub async fn sync_node_metrics() -> anyhow::Result<()> {
    let subnets = fetch_subnets().await?;
    let latest_ts = stable_memory::latest_ts().unwrap_or_default();
    let refresh_ts = latest_ts + 1;

    ic_cdk::println!(
        "Updating node metrics for {} subnets: Latest timestamp persisted: {}  Refreshing metrics from timestamp {}",
        subnets.len(),
        latest_ts,
        refresh_ts
    );
    let subnet_metrics: Vec<(PrincipalId, Vec<NodeMetricsHistoryResponse>)> = fetch_metrics(subnets, refresh_ts).await?;
    let metrics_by_node: BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>> = grouped_by_node(subnet_metrics);
    let principals = metrics_by_node.keys().map(|p| p.0).collect_vec();
    let latest_metrics = stable_memory::latest_metrics(&principals);

    for (node_id, node_metrics_grouped) in metrics_by_node {
        let (initial_proposed_total, initial_failed_total) = latest_metrics
            .get(&node_id.0)
            .map(|metrics| (metrics.num_blocks_proposed_total, metrics.num_blocks_failures_total))
            .unwrap_or((0, 0));
        let node_metrics_storable = node_metrics_storable(node_id, node_metrics_grouped, initial_proposed_total, initial_failed_total);

        for (key, node_metrics) in node_metrics_storable {
            stable_memory::insert_node_metrics(key, node_metrics)
        }
    }

    Ok(())
}

pub struct MetricsQuerier{}
impl MetricsQuerier {
    pub fn get_daily_metrics(&self, rewarding_period: &DateTimeRange) -> BTreeMap<PrincipalId, Vec<DailyNodeMetrics>> {
        let mut daily_metrics: BTreeMap<PrincipalId, Vec<DailyNodeMetrics>> = BTreeMap::new();
        let nodes_metrics = stable_memory::get_metrics_range(
            rewarding_period.start_timestamp_nanos(),
            Some(rewarding_period.end_timestamp_nanos()),
            None,
        );
    
        for ((ts, principal), node_metrics_value) in nodes_metrics {
            let node_id = PrincipalId::from(principal);
            let daily_node_metrics = DailyNodeMetrics::new(
                ts,
                node_metrics_value.subnet_assigned,
                node_metrics_value.num_blocks_proposed,
                node_metrics_value.num_blocks_failed,
            );
    
            daily_metrics.entry(node_id).or_default().push(daily_node_metrics);
        }
        daily_metrics
    }
    
}
