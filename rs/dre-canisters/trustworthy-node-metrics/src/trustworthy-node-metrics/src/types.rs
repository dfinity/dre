use candid::{CandidType, Deserialize, Principal};
use dfn_core::api::PrincipalId;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use serde::Serialize;

pub type PrincipalNodeMetricsHistory = (PrincipalId, Vec<NodeMetricsHistoryResponse>);

// Stored in stable structure
pub type TimestampNanos = u64;
pub type NodeMetricsStoredKey = (TimestampNanos, Principal);
#[derive(Debug, Deserialize, Serialize, CandidType, Clone)]
pub struct NodeMetricsStored {
    pub subnet_assigned: Principal,
    pub num_blocks_proposed_total: u64,
    pub num_blocks_failures_total: u64,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
}

// subnet_node_metrics query call
#[derive(Deserialize, CandidType)]
pub struct SubnetNodeMetricsArgs {
    pub ts: Option<u64>,
    pub subnet_id: Option<Principal>,
}

#[derive(Debug, Deserialize, Serialize, CandidType, Clone)]
pub struct NodeMetrics {
    pub node_id: Principal,
    pub num_blocks_proposed_total: u64,
    pub num_blocks_failures_total: u64,
}

#[derive(Debug, Deserialize, CandidType)]
pub struct SubnetNodeMetricsResponse {
    pub ts: u64,
    pub subnet_id: Principal,
    pub node_metrics: Vec<NodeMetrics>,
}

// node_rewards query call
#[derive(Deserialize, CandidType)]
pub struct NodeRewardsArgs {
    pub from_ts: u64,
    pub to_ts: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, CandidType)]
pub struct DailyNodeMetrics {
    pub ts: u64,
    pub subnet_assigned: Principal,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: f64,
    pub rewards_reduction: f64,
}

#[derive(Debug, Deserialize, CandidType)]
pub struct NodeRewardsResponse {
    pub node_id: Principal,
    pub rewards_no_penalty: f64,
    pub rewards_with_penalty: f64,
    pub daily_node_metrics: Vec<DailyNodeMetrics>,
}
