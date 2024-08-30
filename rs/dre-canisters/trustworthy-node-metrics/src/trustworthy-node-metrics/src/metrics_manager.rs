use std::collections::BTreeMap;

use anyhow::anyhow;
use dfn_core::api::PrincipalId;
use futures::FutureExt;
use ic_base_types::NodeId;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::subnet::v1::SubnetListRecord;
use ic_registry_keys::{make_node_operator_record_key, make_node_record_key};

use crate::stable_memory;
use itertools::Itertools;
use trustworthy_node_metrics_types::types::{NodeMetricsGrouped, NodeMetricsStored, NodeMetricsStoredKey};
use trustworthy_node_metrics_types::types::{SubnetNodeMetricsHistory, TimestampNanos};

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
                .map_err(|(code, msg)| anyhow::anyhow!("Error when calling management canister:\n Code:{:?}\nMsg:{}", code, msg))
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

/// Fetch subnets
///
/// Fetch subnets from the registry canister
async fn fetch_subnets() -> anyhow::Result<Vec<PrincipalId>> {
    let (registry_subnets, _): (SubnetListRecord, _) = ic_nns_common::registry::get_value("subnet_list".as_bytes(), None).await?;
    let subnets = registry_subnets
        .subnets
        .into_iter()
        .map(|subnet_id: Vec<u8>| PrincipalId::try_from(subnet_id))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(subnets)
}

/// Fetch node provider
///
/// Fetch node provider from the registry canister given node_id
async fn fetch_node_provider(node_id: &PrincipalId) -> anyhow::Result<PrincipalId> {
    let node_id = NodeId::from(*node_id);

    let node_record_key = make_node_record_key(node_id);
    let (node_record, _) = ic_nns_common::registry::get_value::<NodeRecord>(node_record_key.as_bytes(), None)
        .await
        .map_err(|e| anyhow!("Error getting the node_record from the registry for node {}. Error: {:?}", node_id, e))?;

    let node_operator_id: PrincipalId = node_record.node_operator_id.try_into()?;
    let node_operator_key = make_node_operator_record_key(node_operator_id);
    let (node_operator_record, _) = ic_nns_common::registry::get_value::<NodeOperatorRecord>(node_operator_key.as_bytes(), None)
        .await
        .map_err(|e| {
            anyhow!(
                "Error getting the node_operator_record from the registry for node  {}. Error: {:?}",
                node_id,
                e
            )
        })?;

    let node_provider_id: PrincipalId = node_operator_record.node_provider_principal_id.try_into()?;

    Ok(node_provider_id)
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

async fn update_node_providers(nodes_principal: Vec<&PrincipalId>) -> anyhow::Result<()> {
    for node_principal in nodes_principal {
        let maybe_node_provider = stable_memory::get_node_provider(&node_principal.0);

        if maybe_node_provider.is_none() {
            match fetch_node_provider(node_principal).await {
                Ok(node_provider_id) => {
                    stable_memory::insert_node_provider(node_principal.0, node_provider_id.0);
                }
                Err(e) => {
                    ic_cdk::println!("Failed to fetch node provider for {:?}: {:?}", node_principal, e);
                }
            }
        }
    }
    Ok(())
}

fn update_node_metrics(metrics_by_node: BTreeMap<PrincipalId, Vec<NodeMetricsGrouped>>) {
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
}

/// Update metrics
pub async fn update_metrics() -> anyhow::Result<()> {
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
    let nodes_principal: Vec<&PrincipalId> = metrics_by_node.keys().collect_vec();

    update_node_providers(nodes_principal).await?;
    update_node_metrics(metrics_by_node);

    Ok(())
}
