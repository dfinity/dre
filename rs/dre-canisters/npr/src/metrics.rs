use crate::metrics_types::{SubnetIdStored, SubnetMetricsStored, SubnetMetricsStoredKey, TimestampNanos};
use crate::storage::VM;
use ic_base_types::SubnetId;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::StableBTreeMap;
use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;
pub type StableSubnetsMetrics = StableBTreeMap<SubnetMetricsStoredKey, SubnetMetricsStored, VM>;
pub type StableLastTimestampPerSubnet = StableBTreeMap<SubnetIdStored, TimestampNanos, VM>;

pub trait MetricsManagerData {
    fn with_subnets_to_retry<R>(f: impl FnOnce(&HashSet<SubnetId>) -> R) -> R;
    fn with_subnets_to_retry_mut<R>(f: impl FnOnce(&mut HashSet<SubnetId>) -> R) -> R;

    fn with_subnets_metrics<R>(f: impl FnOnce(&StableSubnetsMetrics) -> R) -> R;
    fn with_subnets_metrics_mut<R>(f: impl FnOnce(&mut StableSubnetsMetrics) -> R) -> R;

    fn with_last_timestamp_per_subnet<R>(f: impl FnOnce(&StableLastTimestampPerSubnet) -> R) -> R;
    fn with_last_timestamp_per_subnet_mut<R>(f: impl FnOnce(&mut StableLastTimestampPerSubnet) -> R) -> R;
}

pub struct MetricsManager<D: MetricsManagerData> {
    _metrics_manager_data: PhantomData<D>,
}

impl<D: MetricsManagerData> MetricsManager<D> {
    pub async fn retry_metrics_fetching() {
        let subnets_to_retry: Vec<SubnetId> = D::with_subnets_to_retry(|subnets_to_retry| subnets_to_retry.clone().into_iter().collect());

        if !subnets_to_retry.is_empty() {
            ic_cdk::println!("Retrying metrics for subnets: {:?}", subnets_to_retry);
            Self::sync_subnets_metrics(subnets_to_retry).await
        }
    }

    /// Fetch metrics
    ///
    /// Calls to the node_metrics_history endpoint of the management canister for all the subnets
    /// to get updated metrics since refresh_ts.
    async fn fetch_subnets_metrics(
        last_timestamp_per_subnet: &BTreeMap<SubnetId, TimestampNanos>,
    ) -> BTreeMap<SubnetId, CallResult<(Vec<NodeMetricsHistoryResponse>,)>> {
        let mut subnets_node_metrics = Vec::new();

        for (subnet_id, last_metrics_ts) in last_timestamp_per_subnet {
            let refresh_ts = last_metrics_ts + 1;
            ic_cdk::println!(
                "Updating node metrics for subnet {}: Latest timestamp persisted: {}  Refreshing metrics from timestamp {}",
                subnet_id,
                last_metrics_ts,
                refresh_ts
            );

            let contract = NodeMetricsHistoryArgs {
                subnet_id: subnet_id.get(),
                start_at_timestamp_nanos: refresh_ts,
            };

            subnets_node_metrics.push(async move {
                let call_result = ic_cdk::api::call::call_with_payment128::<_, (Vec<NodeMetricsHistoryResponse>,)>(
                    candid::Principal::management_canister(),
                    "node_metrics_history",
                    (contract,),
                    0_u128,
                )
                .await;

                (*subnet_id, call_result)
            });
        }

        futures::future::join_all(subnets_node_metrics).await.into_iter().collect()
    }

    pub async fn sync_subnets_metrics(subnets: Vec<SubnetId>) {
        let last_timestamp_per_subnet = subnets
            .into_iter()
            .map(|subnet| {
                let last_metrics_ts =
                    D::with_last_timestamp_per_subnet(|last_timestamp_per_subnet| last_timestamp_per_subnet.get(&SubnetIdStored(subnet)));
                (subnet, last_metrics_ts.unwrap_or_default())
            })
            .collect();

        let subnets_metrics = Self::fetch_subnets_metrics(&last_timestamp_per_subnet).await;
        for (subnet_id, call_result) in subnets_metrics {
            match call_result {
                Ok((history,)) => {
                    // Update the last timestamp for this subnet.
                    let last_timestamp = history
                        .iter()
                        .map(|entry| entry.timestamp_nanos)
                        .max()
                        .unwrap_or(*last_timestamp_per_subnet.get(&subnet_id).expect("last_timestamp_per_subnet exists"));

                    D::with_last_timestamp_per_subnet_mut(|last_timestamp_per_subnet| {
                        last_timestamp_per_subnet.insert(SubnetIdStored(subnet_id), last_timestamp)
                    });

                    // Insert each fetched metric entry into our node metrics map.
                    D::with_subnets_metrics_mut(|subnets_metrics| {
                        history.into_iter().for_each(|entry| {
                            let key = SubnetMetricsStoredKey {
                                subnet_id,
                                timestamp_nanos: entry.timestamp_nanos,
                            };
                            subnets_metrics.insert(key, SubnetMetricsStored(entry.node_metrics));
                        });
                    });
                    D::with_subnets_to_retry_mut(|subnets_to_retry| subnets_to_retry.remove(&subnet_id));
                }
                Err((code, msg)) => {
                    ic_cdk::println!("Error fetching metrics for subnet {}: CODE: {:?} MSG: {}", subnet_id, code, msg);

                    D::with_subnets_to_retry_mut(|subnets_to_retry| subnets_to_retry.insert(subnet_id));
                }
            }
        }
    }
}
