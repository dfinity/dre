use crate::metrics_types::{KeyRange, SubnetIdKey, SubnetMetricsKeyStored, SubnetMetricsValueStored};
use crate::DAY_IN_SECONDS;
use async_trait::async_trait;
use candid::Principal;
use ic_base_types::SubnetId;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types_private::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::StableBTreeMap;
use itertools::Itertools;
use rewards_calculation::rewards_calculator_results::DayUTC;
use rewards_calculation::types::{DayEnd, NodeMetricsDailyRaw, SubnetMetricsDailyKey, UnixTsNanos};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

pub type RetryCount = u64;

#[async_trait]
pub trait ManagementCanisterClient {
    async fn node_metrics_history(&self, args: NodeMetricsHistoryArgs) -> CallResult<Vec<NodeMetricsHistoryResponse>>;
}

/// Used to interact with remote Management canisters.
pub struct ICCanisterClient;

#[async_trait]
impl ManagementCanisterClient for ICCanisterClient {
    /// Queries the `node_metrics_history` endpoint of the management canisters of the subnet specified
    /// in the 'contract' to fetch daily node metrics.
    async fn node_metrics_history(&self, contract: NodeMetricsHistoryArgs) -> CallResult<Vec<NodeMetricsHistoryResponse>> {
        ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
            Principal::management_canister(),
            "node_metrics_history",
            (contract,),
            0_u128,
        )
        .await
        .map(|(response,)| response)
    }
}

pub struct MetricsManager<Memory>
where
    Memory: ic_stable_structures::Memory,
{
    pub(crate) client: Box<dyn ManagementCanisterClient>,
    pub(crate) subnets_metrics: RefCell<StableBTreeMap<SubnetMetricsKeyStored, SubnetMetricsValueStored, Memory>>,
    pub(crate) subnets_to_retry: RefCell<StableBTreeMap<SubnetIdKey, RetryCount, Memory>>,
    pub(crate) last_timestamp_per_subnet: RefCell<StableBTreeMap<SubnetIdKey, UnixTsNanos, Memory>>,
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
    /// Fetches subnets metrics for the specified subnets from their last timestamp.
    async fn fetch_subnets_metrics(
        &self,
        last_timestamp_per_subnet: &BTreeMap<SubnetId, Option<UnixTsNanos>>,
    ) -> BTreeMap<SubnetId, CallResult<Vec<NodeMetricsHistoryResponse>>> {
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

            subnets_history.push(async move { (*subnet_id, self.client.node_metrics_history(contract).await) });
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
        for (subnet_id, call_result) in subnets_metrics {
            match call_result {
                Ok(subnet_update) => {
                    if subnet_update.is_empty() {
                        ic_cdk::println!("No updates for subnet {}", subnet_id);
                    } else {
                        // Update the last timestamp for this subnet.
                        let last_timestamp = subnet_update.last().map(|metrics| metrics.timestamp_nanos).expect("Not empty");
                        self.last_timestamp_per_subnet.borrow_mut().insert(subnet_id.into(), last_timestamp);

                        for NodeMetricsHistoryResponse {
                            timestamp_nanos,
                            node_metrics,
                        } in subnet_update
                        {
                            self.subnets_metrics.borrow_mut().insert(
                                SubnetMetricsKeyStored {
                                    subnet_id,
                                    ts: timestamp_nanos,
                                },
                                SubnetMetricsValueStored {
                                    nodes_metrics: node_metrics.into_iter().map(|m| m.into()).collect(),
                                },
                            );
                        }
                    }

                    self.subnets_to_retry.borrow_mut().remove(&subnet_id.into());
                }
                Err((_, e)) => {
                    ic_cdk::println!("Error fetching metrics for subnet {}: ERROR: {}", subnet_id, e);

                    // The call failed, will retry fetching metrics for this subnet.
                    let mut retry_count = self.subnets_to_retry.borrow().get(&subnet_id.into()).unwrap_or_default();
                    retry_count += 1;

                    self.subnets_to_retry.borrow_mut().insert(subnet_id.into(), retry_count);
                }
            }
        }
    }
    pub fn daily_metrics_by_subnet(&self, from: DayUTC, to: DayUTC) -> BTreeMap<SubnetMetricsDailyKey, Vec<NodeMetricsDailyRaw>> {
        let mut daily_metrics_by_subnet = BTreeMap::new();
        let start_ts = from.unix_ts_at_day_start();
        let one_day_before_start = start_ts.checked_sub(DAY_IN_SECONDS * 1_000_000_000).unwrap_or_default();
        let first_key = SubnetMetricsKeyStored {
            ts: one_day_before_start,
            ..SubnetMetricsKeyStored::min_key()
        };
        let last_key = SubnetMetricsKeyStored {
            ts: to.unix_ts_at_day_end(),
            ..SubnetMetricsKeyStored::max_key()
        };

        let mut subnets_metrics_by_day: BTreeMap<_, _> = self
            .subnets_metrics
            .borrow()
            .range(first_key..=last_key)
            .into_group_map_by(|(k, _)| DayEnd::from(k.ts))
            .into_iter()
            .collect();

        let mut last_total_metrics: HashMap<_, _> = HashMap::new();
        if let Some((ts, _)) = subnets_metrics_by_day.first_key_value() {
            if ts.get() < start_ts {
                last_total_metrics = subnets_metrics_by_day
                    .pop_first()
                    .unwrap()
                    .1
                    .into_iter()
                    .flat_map(|(k, v)| {
                        v.nodes_metrics.into_iter().map(move |node_metrics| {
                            (
                                (k.subnet_id, node_metrics.node_id),
                                (node_metrics.num_blocks_proposed_total, node_metrics.num_blocks_failed_total),
                            )
                        })
                    })
                    .collect();
            }
        };

        for (_, subnets_metrics) in subnets_metrics_by_day {
            // current_total_metrics holds the total metrics for the current day per node per subnet.
            // It will be used to calculate the daily metrics for each node the next day.
            let mut current_total_metrics: HashMap<_, _> = HashMap::new();
            for (k, v) in subnets_metrics {
                let daily_nodes_metrics: Vec<NodeMetricsDailyRaw> = v
                    .nodes_metrics
                    .into_iter()
                    .map(|node| {
                        let (last_proposed_total, last_failed_total) = last_total_metrics.remove(&(k.subnet_id, node.node_id)).unwrap_or_default();
                        current_total_metrics.insert(
                            (k.subnet_id, node.node_id),
                            (node.num_blocks_proposed_total, node.num_blocks_failed_total),
                        );

                        NodeMetricsDailyRaw {
                            node_id: node.node_id,
                            num_blocks_proposed: node.num_blocks_proposed_total - last_proposed_total,
                            num_blocks_failed: node.num_blocks_failed_total - last_failed_total,
                        }
                    })
                    .collect();

                daily_metrics_by_subnet.insert(k.into(), daily_nodes_metrics);
            }
            last_total_metrics = current_total_metrics;
        }

        daily_metrics_by_subnet
    }
}

#[cfg(test)]
mod tests;
