use ahash::AHashMap;
use ic_base_types::PrincipalId;

use crate::v1_logs::RewardsLog;

pub type NodeMultiplierStats = (PrincipalId, MultiplierStats);
pub type RewardablesWithNodesMetrics = (AHashMap<RegionNodeTypeCategory, u32>, AHashMap<Node, Vec<DailyPerformanceMetrics>>);
pub type RegionNodeTypeCategory = (String, String);
pub type TimestampNanos = u64;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Node {
    pub node_id: PrincipalId,
    pub node_provider_id: PrincipalId,
    pub region: String,
    pub node_type: String,
}

#[derive(Clone)]
pub struct DailyPerformanceMetrics {
    pub ts: u64,
    pub subnet_assigned: PrincipalId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
}

pub struct MultiplierStats {
    pub days_assigned: u64,
    pub days_unassigned: u64,
    pub rewards_reduction: f64,
    pub blocks_failed: u64,
    pub blocks_proposed: u64,
    pub blocks_total: u64,
    pub failure_rate: f64,
}

pub struct RewardsPerNodeProvider {
    pub rewards_per_node_provider: AHashMap<PrincipalId, (Rewards, Vec<NodeMultiplierStats>)>,
    pub rewards_log_per_node_provider: AHashMap<PrincipalId, RewardsLog>,
}

pub struct Rewards {
    pub xdr_permyriad: u64,
    pub xdr_permyriad_no_reduction: u64,
}
