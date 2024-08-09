use std::collections::BTreeMap;

use anyhow::Ok;
use dfn_core::api::PrincipalId;
use futures::FutureExt;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_protobuf::registry::subnet::v1::SubnetListRecord;

use crate::types::{NodeMetricsStored, NodeMetricsStoredKey};
use crate::{
    stable_memory,
    types::{PrincipalNodeMetricsHistory, TimestampNanos},
};

fn store_results(results: BTreeMap<NodeMetricsStoredKey, NodeMetricsStored>) {
    for (timestamp, storable) in results {
        stable_memory::insert(timestamp, storable)
    }
}

fn get_daily_proposed_failed(new_node_metrics: &ic_management_canister_types::NodeMetrics) -> (u64, u64) {
    let principal = new_node_metrics.node_id.0;
    let existing_metrics = stable_memory::latest_metrics(principal);
    let new_failed_total = new_node_metrics.num_block_failures_total;
    let new_proposed_total = new_node_metrics.num_blocks_proposed_total;

    let (num_blocks_proposed, num_blocks_failed) = match existing_metrics {
        Some(existing_metrics_value) => {
            let existing_failed_total = existing_metrics_value.num_blocks_failures_total;
            let existing_proposed_total = existing_metrics_value.num_blocks_proposed_total;

            if existing_failed_total > new_failed_total || existing_proposed_total > new_proposed_total {
                // This is the case when the node gets redeployed
                (new_proposed_total, new_failed_total)
            } else {
                (new_proposed_total - existing_proposed_total, new_failed_total - existing_failed_total)
            }
        }
        None => (new_proposed_total, new_failed_total),
    };

    (num_blocks_proposed, num_blocks_failed)
}

/// Transform metrics
///
/// Groups the metrics received by timestamp to fit the "storable" format
fn transform_metrics(subnets_metrics: Vec<PrincipalNodeMetricsHistory>) -> BTreeMap<NodeMetricsStoredKey, NodeMetricsStored> {
    let mut results = BTreeMap::new();

    for (subnet_id, subnet_metrics) in subnets_metrics {
        for ts_node_metrics in subnet_metrics {
            let ts: TimestampNanos = ts_node_metrics.timestamp_nanos;

            for node_metrics in ts_node_metrics.node_metrics {
                let principal = node_metrics.node_id.0;
                let node_metrics_key = (ts, principal);

                let (new_blocks_proposed, new_blocks_failed) = get_daily_proposed_failed(&node_metrics);

                let node_metrics_value = NodeMetricsStored {
                    subnet_assigned: subnet_id.0,
                    num_blocks_proposed_total: node_metrics.num_blocks_proposed_total,
                    num_blocks_failures_total: node_metrics.num_block_failures_total,
                    num_blocks_proposed: new_blocks_proposed,
                    num_blocks_failed: new_blocks_failed,
                };

                results.insert(node_metrics_key, node_metrics_value);
            }
        }
    }
    results
}

/// Fetch metrics
///
/// Calls to the node_metrics_history endpoint of the management canister for all the subnets
/// to get updated metrics since refresh_ts.
async fn fetch_metrics(subnets: Vec<PrincipalId>, refresh_ts: TimestampNanos) -> anyhow::Result<Vec<PrincipalNodeMetricsHistory>> {
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

pub async fn update_metrics() -> anyhow::Result<()> {
    let subnets = fetch_subnets().await?;
    let latest_ts = stable_memory::latest_ts().unwrap_or_default();
    let refresh_ts = latest_ts + 1;

    ic_cdk::println!(
        "Updating node metrics for {} subnets:\nLatest timestamp persisted: {}\nRefreshing metrics from timestamp {}",
        subnets.len(),
        latest_ts,
        refresh_ts
    );

    let metrics = fetch_metrics(subnets, refresh_ts).await?;

    let results = transform_metrics(metrics);

    store_results(results);

    Ok(())
}
