use std::{borrow::Cow, fmt};

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use dfn_core::api::PrincipalId;
use ic_management_canister_types::NodeMetricsHistoryResponse;
use ic_protobuf::registry::node_rewards::v2::NodeRewardRates;
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;

pub type SubnetNodeMetricsHistory = (PrincipalId, Vec<NodeMetricsHistoryResponse>);
pub type NodeMetricsGrouped = (u64, PrincipalId, ic_management_canister_types::NodeMetrics);

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

const MAX_VALUE_SIZE_BYTES_NODE_METRICS: u32 = 102;

impl Storable for NodeMetricsStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE_BYTES_NODE_METRICS,
        is_fixed_size: false,
    };
}

#[derive(Debug, Deserialize, Serialize, CandidType, Clone)]
pub struct NodeRewardRatesStored {
    pub rewards_rates: NodeRewardRates,
}

const MAX_VALUE_SIZE_BYTES_REWARD_RATES: u32 = 133;

impl Storable for NodeRewardRatesStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE_BYTES_REWARD_RATES,
        is_fixed_size: false,
    };
}

#[derive(Debug, Deserialize, Serialize, CandidType, Clone)]
pub struct NodeMetadataStored {
    pub node_provider_id: Principal,
    pub node_provider_name: Option<String>,
}

impl Storable for NodeMetadataStored {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE_BYTES_NODE_METRICS,
        is_fixed_size: false,
    };
}

#[derive(Debug, Deserialize, Serialize, CandidType, Clone)]
pub struct NodeMetadataStoredV2 {
    pub node_provider_id: Principal,
    pub node_provider_name: Option<String>,
    pub dc_id: String,
    pub region: String,
    pub node_type: String,
}

const MAX_VALUE_SIZE_BYTES_NODE_METADATA: u32 = 140;

impl Storable for NodeMetadataStoredV2 {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE_BYTES_NODE_METADATA,
        is_fixed_size: false,
    };
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
    pub node_id: Option<Principal>,
    pub node_provider_id: Option<Principal>,
}

#[derive(Debug, Clone, Deserialize, Serialize, CandidType)]
pub struct DailyNodeMetrics {
    pub ts: u64,
    pub subnet_assigned: Principal,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,

    /// The failure rate of the node for the day, calculated as a ratio of
    /// `num_blocks_failed` to `num_blocks_total` = `num_blocks_failed` + `num_blocks_proposed`.
    /// This value ranges from 0.0 (no failures) to 1.0 (all blocks failed).
    pub failure_rate: f64,
}

impl fmt::Display for DailyNodeMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "timestamp_nanoseconds: {}, num_blocks_proposed: {},  num_blocks_failed: {}",
            self.ts, self.num_blocks_proposed, self.num_blocks_failed
        )
    }
}

impl DailyNodeMetrics {
    pub fn new(ts: TimestampNanos, subnet_assignment: Principal, proposed_blocks: u64, failed_blocks: u64) -> Self {
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

#[derive(Debug, Deserialize, CandidType)]
pub struct RewardsComputationResult {
    pub rewards_percent: f64,
    pub rewards_reduction: f64,
    pub blocks_failed: u64,
    pub blocks_proposed: u64,
    pub blocks_total: u64,
    pub failure_rate: f64,
    pub computation_log: String,
}

#[derive(Debug, Deserialize, CandidType)]
pub struct NodeRewardsResponse {
    pub node_id: Principal,
    pub node_provider_id: Principal,
    pub daily_node_metrics: Vec<DailyNodeMetrics>,
    pub rewards_computation: RewardsComputationResult,
}

#[derive(Debug, Deserialize, CandidType)]
pub struct NodeProviderMapping {
    pub node_id: Principal,
    pub node_provider_id: Principal,
}

#[derive(Debug, Deserialize, CandidType)]
pub struct NodeMetadata {
    pub node_id: Principal,
    pub node_provider_id: Principal,
    pub node_provider_name: Option<String>,
}
