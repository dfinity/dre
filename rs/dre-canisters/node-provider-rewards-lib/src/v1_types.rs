use std::collections::BTreeMap;

use ic_base_types::PrincipalId;

use crate::v1_logs::RewardsPerNodeProviderLog;

pub type NodeMultiplierStats = (PrincipalId, MultiplierStats);
pub type RewardablesWithMetrics = (BTreeMap<RegionNodeTypeCategory, u32>, BTreeMap<Node, Vec<DailyNodeMetrics>>);
pub type RegionNodeTypeCategory = (String, String);
pub type TimestampNanos = u64;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub node_id: PrincipalId,
    pub node_provider_id: PrincipalId,
    pub region: String,
    pub node_type: String,
}

#[derive(Clone)]
pub struct DailyNodeMetrics {
    pub ts: u64,
    pub subnet_assigned: PrincipalId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,

    /// The failure rate of the node for the day, calculated as a ratio of
    /// `num_blocks_failed` to `num_blocks_total` = `num_blocks_failed` + `num_blocks_proposed`.
    /// This value ranges from 0.0 (no failures) to 1.0 (all blocks failed).
    pub failure_rate: f64,
}

impl DailyNodeMetrics {
    pub fn new(ts: TimestampNanos, subnet_assignment: PrincipalId, proposed_blocks: u64, failed_blocks: u64) -> Self {
        let total_blocks = failed_blocks + proposed_blocks;
        let failure_rate = if total_blocks == 0 {
            0.0
        } else {
            failed_blocks as f64 / total_blocks as f64
        };

        DailyNodeMetrics {
            ts,
            subnet_assigned: subnet_assignment,
            num_blocks_proposed: proposed_blocks,
            num_blocks_failed: failed_blocks,
            failure_rate,
        }
    }
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
    pub rewards_per_node_provider: BTreeMap<PrincipalId, (Rewards, Vec<NodeMultiplierStats>)>,
    pub computation_log: BTreeMap<PrincipalId, RewardsPerNodeProviderLog>,
}

pub struct Rewards {
    pub xdr_permyriad: u64,
    pub xdr_permyriad_no_reduction: u64,
}
