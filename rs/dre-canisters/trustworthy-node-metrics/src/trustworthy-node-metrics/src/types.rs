use candid::{CandidType, Deserialize, Principal};
use dfn_core::api::PrincipalId;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use serde::Serialize;

pub type TimestampNanos = u64;
pub type PrincipalNodeMetricsHistory = (PrincipalId, Vec<NodeMetricsHistoryResponse>);

#[derive(Debug, Deserialize, Serialize, CandidType, Clone)]
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

#[derive(Deserialize, Serialize)]
pub struct SubnetNodeMetricsStorable(pub Vec<SubnetNodeMetrics>);

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

#[derive(Debug)]
pub struct RewardsMetrics {
    pub rewards_standard: u64,
    pub node_metrics: Vec<NodeMetrics>,
}

#[derive(Debug, Deserialize, Serialize, CandidType)]
pub struct Rewards {
    pub rewards_standard: f64,
    pub rewards_with_penalty: f64,
}

#[derive(Debug, Deserialize, CandidType)]
pub struct NodeRewardsResponse {
    pub node_id: Principal,
    pub node_rewards: Rewards,
}

#[derive(Deserialize, CandidType)]
pub struct NodeRewardsArgs {
    pub from_ts: u64,
    pub to_ts: u64,
}
