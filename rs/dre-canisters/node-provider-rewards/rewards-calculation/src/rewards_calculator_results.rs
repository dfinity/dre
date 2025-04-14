use crate::types::{DayEndNanos, RewardPeriod, RewardPeriodError, TimestampNanos};
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeCategory {
    pub region: String,
    pub node_type: String,
}

/// Represents the daily metrics recorded for a node.
#[derive(Clone, PartialEq, Debug)]
pub struct NodeMetricsDailyProcessed {
    pub ts: DayEndNanos,
    pub subnet_assigned: SubnetId,
    pub subnet_assigned_fr: Decimal,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub original_fr: Decimal,
    pub relative_fr: Decimal,
}

#[derive(Debug, Default)]
pub struct NodeResults {
    pub region: String,
    pub node_type: String,
    pub daily_metrics: Vec<NodeMetricsDailyProcessed>,
    // None if node unassigned in reward period
    pub avg_relative_fr: Option<Decimal>,
    pub avg_relative_extrapolated_fr: Decimal,
    pub rewards_reduction: Decimal,
    pub performance_multiplier: Decimal,
    pub adjusted_rewards: Decimal,
}

#[derive(Default, Debug)]
pub struct RewardsCalculatorResults {
    pub base_rewards_by_category: BTreeMap<NodeCategory, Decimal>,
    pub results_by_node: BTreeMap<NodeId, NodeResults>,
    pub extrapolated_fr: Decimal,
    pub rewards_total: Decimal,
}

#[derive(Debug, PartialEq)]
pub enum RewardCalculatorError {
    RewardPeriodError(RewardPeriodError),
    EmptyMetrics,
    SubnetMetricsOutOfRange {
        subnet_id: SubnetId,
        timestamp: TimestampNanos,
        reward_period: RewardPeriod,
    },
    DuplicateMetrics(SubnetId, DayEndNanos),
}

impl From<RewardPeriodError> for RewardCalculatorError {
    fn from(err: RewardPeriodError) -> Self {
        RewardCalculatorError::RewardPeriodError(err)
    }
}

impl Error for RewardCalculatorError {}

impl fmt::Display for RewardCalculatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardCalculatorError::EmptyMetrics => {
                write!(f, "No daily_metrics_by_node")
            }
            RewardCalculatorError::SubnetMetricsOutOfRange {
                subnet_id,
                timestamp,
                reward_period,
            } => {
                write!(
                    f,
                    "Node {} has metrics outside the reward period: timestamp: {} not in {}",
                    subnet_id, timestamp, reward_period
                )
            }
            RewardCalculatorError::DuplicateMetrics(subnet_id, ts) => {
                write!(f, "Subnet {} has multiple metrics for the same node at ts {}", subnet_id, ts.get())
            }
            RewardCalculatorError::RewardPeriodError(err) => {
                write!(f, "Reward period error: {}", err)
            }
        }
    }
}
