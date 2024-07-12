use candid::{CandidType, Deserialize, Principal};
use dfn_core::api::PrincipalId;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use serde::Serialize;

pub type TimestampNanos = u64;
pub type PrincipalNodeMetricsHistory = (PrincipalId, Vec<NodeMetricsHistoryResponse>);

#[derive(Clone, Debug, Deserialize, Serialize, CandidType)]
pub struct NodeMetrics {
    pub node_id: Principal,
    pub num_blocks_proposed_total: u64,
    pub num_block_failures_total: u64,
}

#[derive(Deserialize, Serialize, CandidType)]
pub struct SubnetMetrics {
    pub subnet_id: Principal,
    pub node_metrics: Vec<NodeMetrics>,
}

#[derive(Deserialize, CandidType)]
pub struct MetricsResponse {
    pub ts: u64,
    pub subnets_metrics: Vec<SubnetMetrics>,
}

#[derive(Deserialize, CandidType)]
pub struct SubnetMetricsResponse {
    pub ts: u64,
    pub node_metrics: Vec<NodeMetrics>,
}

#[derive(Deserialize, Serialize)]
pub struct SubnetsMetricsStorable(pub Vec<SubnetMetrics>);
