use std::borrow::Cow;

use candid::{CandidType, Deserialize, Principal};
use dfn_core::api::PrincipalId;
use ic_management_canister_types::NodeMetrics as ICManagementNodeMetrics;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use ic_stable_structures::{storable::Bound, Storable};
use itertools::Itertools;
use serde::Serialize;

pub type TimestampNanos = u64;
pub type PrincipalNodeMetricsHistory = (PrincipalId, Vec<NodeMetricsHistoryResponse>);

#[derive(Debug, Deserialize, Serialize, CandidType)]
pub struct NodeMetrics {
    pub node_id: Principal,
    pub num_blocks_proposed_total: u64,
    pub num_block_failures_total: u64,
}

#[derive(Deserialize, Serialize, CandidType)]
pub struct SubnetNodeMetrics {
    pub subnet_id: Principal,
    pub node_metrics: Vec<NodeMetrics>,
}

impl SubnetNodeMetrics {
    pub fn new(subnet_id: PrincipalId, subnet_metrics: Vec<ICManagementNodeMetrics>) -> Self {
        let node_metrics = subnet_metrics.into_iter().map(|node_metrics| node_metrics.into()).collect_vec();

        Self {
            subnet_id: subnet_id.0,
            node_metrics,
        }
    }
}

impl From<ICManagementNodeMetrics> for NodeMetrics {
    fn from(node_metrics: ICManagementNodeMetrics) -> Self {
        Self {
            node_id: node_metrics.node_id.0,
            num_block_failures_total: node_metrics.num_block_failures_total,
            num_blocks_proposed_total: node_metrics.num_blocks_proposed_total,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct SubnetNodeMetricsStorable(pub Vec<SubnetNodeMetrics>);

impl Storable for SubnetNodeMetricsStorable {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        ciborium::ser::into_writer(self, &mut buf).expect("failed to encode SubnetsMetricsStorable");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        ciborium::de::from_reader(&bytes[..]).expect("failed to decode SubnetsMetricsStorable")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Debug, Deserialize, CandidType)]
pub struct SubnetNodeMetricsResponse {
    pub ts: u64,
    pub subnet_id: Principal,
    pub node_metrics: Vec<NodeMetrics>,
}

#[derive(Deserialize, CandidType)]
pub struct SubnetNodeMetricsArgs {
    pub ts: Option<u64>,
    pub subnet_id: Option<Principal>,
}
