use candid::{CandidType, Decode, Encode};
use ic_base_types::SubnetId;
use ic_cdk::api::call::CallResult;
use ic_management_canister_types::NodeMetrics;
use ic_management_canister_types::{NodeMetricsHistoryArgs, NodeMetricsHistoryResponse};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{StableBTreeMap, StableVec, Storable};
use itertools::Itertools;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

pub(crate) type TimestampNanos = u64;

// Maximum sizes for the storable types chosen as result of test `max_bound_size`
const MAX_BYTES_SUBNET_ID_STORED: u32 = 38;
const MAX_BYTES_NODE_METRICS_STORED_KEY: u32 = 60;
const MAX_BYTES_NODE_METRICS_STORED: u32 = 76;

#[test]
fn max_bound_size() {
    use candid::Principal;
    use ic_base_types::PrincipalId;

    let max_principal_id = PrincipalId::from(Principal::from_slice(&[0xFF; 29]));

    let max_subnet_id_stored = SubnetIdStored(max_principal_id.into());
    let max_node_metrics_stored_key = SubnetMetricsStoredKey {
        timestamp_nanos: u64::MAX,
        subnet_id: max_principal_id.into(),
    };
    let max_node_metrics_stored = SubnetMetricsStored(vec![NodeMetrics {
        node_id: max_principal_id,
        num_blocks_proposed_total: u64::MAX,
        num_block_failures_total: u64::MAX,
    }]);

    assert_eq!(max_subnet_id_stored.to_bytes().len(), MAX_BYTES_SUBNET_ID_STORED as usize);

    assert_eq!(max_node_metrics_stored_key.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED_KEY as usize);

    assert_eq!(max_node_metrics_stored.to_bytes().len(), MAX_BYTES_NODE_METRICS_STORED as usize);
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SubnetIdStored(pub(crate) SubnetId);
impl Storable for SubnetIdStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_SUBNET_ID_STORED,
        is_fixed_size: false,
    };
}

#[derive(Clone, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SubnetMetricsStoredKey {
    pub timestamp_nanos: TimestampNanos,
    pub subnet_id: SubnetId,
}

impl Storable for SubnetMetricsStoredKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_BYTES_NODE_METRICS_STORED_KEY,
        is_fixed_size: false,
    };
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub(crate) struct SubnetMetricsStored(pub(crate) Vec<NodeMetrics>);

impl Storable for SubnetMetricsStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        // This size supports subnets with max 400 nodes
        max_size: MAX_BYTES_NODE_METRICS_STORED * 400,
        is_fixed_size: false,
    };
}

pub struct MetricsManager<Memory: ic_stable_structures::Memory> {
    pub subnets_to_retry: HashSet<SubnetId>,
    subnets_metrics: StableBTreeMap<SubnetMetricsStoredKey, SubnetMetricsStored, Memory>,
    last_timestamp_per_subnet: StableBTreeMap<SubnetIdStored, TimestampNanos, Memory>,
}

impl<Memory: ic_stable_structures::Memory> MetricsManager<Memory> {
    pub fn init(subnets_metrics_memory: Memory, last_timestamp_per_subnet_memory: Memory, subnets_to_retry: HashSet<SubnetId>) -> Self {
        Self {
            subnets_to_retry,
            subnets_metrics: StableBTreeMap::init(subnets_metrics_memory),
            last_timestamp_per_subnet: StableBTreeMap::init(last_timestamp_per_subnet_memory),
        }
    }

    /// Fetch metrics
    ///
    /// Calls to the node_metrics_history endpoint of the management canister for all the subnets
    /// to get updated metrics since refresh_ts.
    async fn fetch_subnets_metrics(
        &self,
        last_timestamp_per_subnet: BTreeMap<SubnetId, TimestampNanos>,
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

                (subnet_id, call_result)
            });
        }

        futures::future::join_all(subnets_node_metrics).await.into_iter().collect()
    }

    pub async fn sync_subnets_metrics(&mut self, subnets: &Vec<SubnetId>) {
        let last_timestamp_per_subnet = subnets
            .clone()
            .into_iter()
            .map(|subnet| {
                let last_metrics_ts = self.last_timestamp_per_subnet.get(&SubnetIdStored(subnet));
                (subnet, last_metrics_ts.unwrap_or_default())
            })
            .collect();

        let subnets_metrics = self.fetch_subnets_metrics(last_timestamp_per_subnet).await;
        for (subnet_id, call_result) in subnets_metrics {
            match call_result {
                Ok((history,)) => {
                    // Update the last timestamp for this subnet.
                    let last_timestamp = history.last().map(|entry| entry.timestamp_nanos).unwrap_or_default();
                    self.last_timestamp_per_subnet.insert(SubnetIdStored(subnet_id), last_timestamp);

                    // Insert each fetched metric entry into our node metrics map.
                    history.into_iter().for_each(|entry| {
                        let key = SubnetMetricsStoredKey {
                            subnet_id,
                            timestamp_nanos: entry.timestamp_nanos,
                        };
                        self.subnets_metrics.insert(key, SubnetMetricsStored(entry.node_metrics));
                    });

                    self.subnets_to_retry.remove(&subnet_id);
                }
                Err((code, msg)) => {
                    ic_cdk::println!(
                        "Error fetching metrics for subnet {}: CODE: {:?} MSG: {}. Will retry every hour.",
                        subnet_id,
                        code,
                        msg
                    );
                    self.subnets_to_retry.insert(subnet_id);
                }
            }
        }
    }
}
