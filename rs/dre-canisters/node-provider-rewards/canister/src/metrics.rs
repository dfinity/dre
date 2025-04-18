use crate::metrics_types::{KeyRange, NodeMetricsDailyStored, SubnetIdKey, SubnetMetricsDailyKeyStored, SubnetMetricsDailyValueStored};
use async_trait::async_trait;
use candid::Principal;
use ic_base_types::SubnetId;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types_private::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::StableBTreeMap;
use rewards_calculation::types::{NodeMetricsDaily, SubnetMetricsDailyKey, TimestampNanos};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

pub type RetryCount = u64;

#[async_trait]
pub trait ManagementCanisterClient {
    async fn node_metrics_history(&self, args: NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)>;
}

/// Used to interact with remote Management canisters.
pub struct ICCanisterClient;

#[async_trait]
impl ManagementCanisterClient for ICCanisterClient {
    /// Queries the `node_metrics_history` endpoint of the management canisters of the subnet specified
    /// in the 'contract' to fetch daily node metrics.
    async fn node_metrics_history(&self, contract: NodeMetricsHistoryArgs) -> CallResult<(Vec<NodeMetricsHistoryResponse>,)> {
        ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
            Principal::management_canister(),
            "node_metrics_history",
            (contract,),
            0_u128,
        )
        .await
    }
}

pub struct MetricsManager<Memory>
where
    Memory: ic_stable_structures::Memory,
{
    pub(crate) client: Box<dyn ManagementCanisterClient>,
    pub(crate) subnets_metrics: RefCell<StableBTreeMap<SubnetMetricsDailyKeyStored, SubnetMetricsDailyValueStored, Memory>>,
    pub(crate) subnets_to_retry: RefCell<StableBTreeMap<SubnetIdKey, RetryCount, Memory>>,
    pub(crate) last_timestamp_per_subnet: RefCell<StableBTreeMap<SubnetIdKey, TimestampNanos, Memory>>,
}

impl<Memory> MetricsManager<Memory>
where
    Memory: ic_stable_structures::Memory + 'static,
{
    pub async fn retry_failed_subnets(&self) {
        let subnets_to_retry: Vec<SubnetId> = self.subnets_to_retry.borrow().keys().map(|key| key.0).collect();

        if !subnets_to_retry.is_empty() {
            ic_cdk::println!("Retrying metrics for subnets: {:?}", subnets_to_retry);
            self.update_subnets_metrics(subnets_to_retry).await;
        }
    }

    /// Update the daily metrics for each node in the subnet.
    fn update_nodes_metrics_daily(
        &self,
        subnet_id: SubnetId,
        last_stored_ts: Option<TimestampNanos>,
        mut subnet_update: Vec<NodeMetricsHistoryResponse>,
    ) {
        // Extract initial total metrics for each node in the subnet.
        let mut initial_total_metrics_per_node: HashMap<_, _> = HashMap::new();

        subnet_update.sort_by_key(|metrics| metrics.timestamp_nanos);
        if let Some(first_metrics) = subnet_update.first() {
            if Some(first_metrics.timestamp_nanos) == last_stored_ts {
                initial_total_metrics_per_node = subnet_update
                    .remove(0)
                    .node_metrics
                    .iter()
                    .map(|node_metrics| {
                        (
                            node_metrics.node_id,
                            (node_metrics.num_blocks_proposed_total, node_metrics.num_block_failures_total),
                        )
                    })
                    .collect();
            }
        };

        let mut running_total_metrics_per_node = initial_total_metrics_per_node;
        for one_day_update in subnet_update {
            let key = SubnetMetricsDailyKeyStored {
                subnet_id,
                ts: one_day_update.timestamp_nanos,
            };

            let daily_nodes_metrics: Vec<_> = one_day_update
                .node_metrics
                .into_iter()
                .map(|node_metrics| {
                    let current_proposed_total = node_metrics.num_blocks_proposed_total;
                    let current_failed_total = node_metrics.num_block_failures_total;

                    let (mut running_proposed_total, mut running_failed_total) = running_total_metrics_per_node
                        .remove(&node_metrics.node_id)
                        // Default is needed if the node joined the subnet after last_stored_ts.
                        .unwrap_or_default();

                    // This can happen if the node was redeployed.
                    if running_proposed_total > current_proposed_total || running_failed_total > current_failed_total {
                        running_proposed_total = 0;
                        running_failed_total = 0;
                    };

                    // Update the total metrics for the next iteration.
                    running_total_metrics_per_node.insert(node_metrics.node_id, (current_proposed_total, current_failed_total));

                    NodeMetricsDailyStored {
                        node_id: node_metrics.node_id.into(),
                        num_blocks_proposed: current_proposed_total - running_proposed_total,
                        num_blocks_failed: current_failed_total - running_failed_total,
                    }
                })
                .collect();

            self.subnets_metrics.borrow_mut().insert(
                key,
                SubnetMetricsDailyValueStored {
                    nodes_metrics: daily_nodes_metrics,
                },
            );
        }
    }

    /// Fetches subnets metrics for the specified subnets from their last timestamp.
    async fn fetch_subnets_metrics(
        &self,
        last_timestamp_per_subnet: &BTreeMap<SubnetId, Option<TimestampNanos>>,
    ) -> BTreeMap<(SubnetId, Option<TimestampNanos>), CallResult<(Vec<NodeMetricsHistoryResponse>,)>> {
        let mut subnets_history = Vec::new();

        for (subnet_id, last_stored_ts) in last_timestamp_per_subnet {
            let refresh_ts = last_stored_ts.unwrap_or_default();
            // For nodes assigned to the subnet before the current update, the TOTAL metrics at last_stored_ts
            // are required since only DAILY metrics for each node are stored.
            ic_cdk::println!(
                "Updating node metrics for subnet {}: Refreshing metrics from timestamp {}",
                subnet_id,
                refresh_ts
            );

            let contract = NodeMetricsHistoryArgs {
                subnet_id: subnet_id.get(),
                start_at_timestamp_nanos: refresh_ts,
            };

            subnets_history.push(async move { ((*subnet_id, *last_stored_ts), self.client.node_metrics_history(contract).await) });
        }

        futures::future::join_all(subnets_history).await.into_iter().collect()
    }

    /// Updates the stored subnets metrics from remote management canisters.
    ///
    /// This function fetches the nodes metrics for the given subnets from the management canisters
    /// updating the local metrics with the fetched metrics.
    pub async fn update_subnets_metrics(&self, subnets: Vec<SubnetId>) {
        let last_timestamp_per_subnet: BTreeMap<SubnetId, _> = subnets
            .into_iter()
            .map(|subnet| {
                let last_timestamp = self.last_timestamp_per_subnet.borrow().get(&SubnetIdKey(subnet));

                (subnet, last_timestamp)
            })
            .collect();

        let subnets_metrics = self.fetch_subnets_metrics(&last_timestamp_per_subnet).await;
        for ((subnet_id, last_stored_ts), call_result) in subnets_metrics {
            match call_result {
                Ok((subnet_update,)) => {
                    if subnet_update.is_empty() {
                        ic_cdk::println!("No updates for subnet {}", subnet_id);
                    } else {
                        // Update the last timestamp for this subnet.
                        let last_timestamp = subnet_update.last().map(|metrics| metrics.timestamp_nanos).expect("Not empty");
                        self.last_timestamp_per_subnet.borrow_mut().insert(subnet_id.into(), last_timestamp);

                        self.update_nodes_metrics_daily(subnet_id, last_stored_ts, subnet_update);
                    }

                    self.subnets_to_retry.borrow_mut().remove(&subnet_id.into());
                }
                Err((code, msg)) => {
                    ic_cdk::println!("Error fetching metrics for subnet {}: CODE: {:?} MSG: {}", subnet_id, code, msg);

                    // The call failed, will retry fetching metrics for this subnet.
                    let mut retry_count = self.subnets_to_retry.borrow().get(&subnet_id.into()).unwrap_or_default();
                    retry_count += 1;

                    self.subnets_to_retry.borrow_mut().insert(subnet_id.into(), retry_count);
                }
            }
        }
    }
    pub fn daily_metrics_by_subnet(&self, start_ts: TimestampNanos, end_ts: TimestampNanos) -> HashMap<SubnetMetricsDailyKey, Vec<NodeMetricsDaily>> {
        let first_key = SubnetMetricsDailyKeyStored {
            ts: start_ts,
            ..SubnetMetricsDailyKeyStored::min_key()
        };
        let last_key = SubnetMetricsDailyKeyStored {
            ts: end_ts,
            ..SubnetMetricsDailyKeyStored::max_key()
        };

        // Group node metrics by node_id within the given time range
        self.subnets_metrics
            .borrow()
            .range(first_key..=last_key)
            .map(|(key, value)| (key.into(), value.into()))
            .collect()
    }
}

#[cfg(test)]
mod tests;
