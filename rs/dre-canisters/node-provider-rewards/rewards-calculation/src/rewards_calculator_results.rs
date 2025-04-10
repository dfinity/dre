use crate::types::{DayEndNanos, NodeMetricsDailyProcessed, RewardPeriod, RewardPeriodError, SubnetFailureRateDaily, TimestampNanos};
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

#[derive(Default, Debug)]
pub struct NodeResults {
    pub region: String,
    pub node_type: String,
    pub daily_metrics_processed: Vec<NodeMetricsDailyProcessed>,
    pub average_relative_fr: Decimal,
    pub average_extrapolated_fr: Decimal,
    pub rewards_reduction: Decimal,
    pub performance_multiplier: Decimal,
    pub base_rewards: Decimal,
    pub adjusted_rewards: Decimal,
}

#[derive(Default, Debug)]
pub struct RewardsCalculatorResults {
    pub daily_subnets_fr: BTreeMap<SubnetId, Vec<SubnetFailureRateDaily>>,
    pub nodes_results: BTreeMap<NodeId, NodeResults>,
    pub rewards_by_category: BTreeMap<NodeCategory, Decimal>,
    pub extrapolated_fr: Decimal,
    pub rewards_total: Decimal,
}

#[derive(Debug, PartialEq)]
pub enum RewardCalculatorError {
    RewardPeriodError(RewardPeriodError),
    EmptyMetrics,
    NodeMetricsOutOfRange {
        node_id: NodeId,
        timestamp: TimestampNanos,
        reward_period: RewardPeriod,
    },
    DuplicateMetrics(NodeId),
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
            RewardCalculatorError::NodeMetricsOutOfRange {
                node_id,
                timestamp,
                reward_period,
            } => {
                write!(
                    f,
                    "Node {} has metrics outside the reward period: timestamp: {} not in {}",
                    node_id, timestamp, reward_period
                )
            }
            RewardCalculatorError::DuplicateMetrics(node_id) => {
                write!(f, "Node {} has multiple metrics for the same day", node_id)
            }
            RewardCalculatorError::RewardPeriodError(err) => {
                write!(f, "Reward period error: {}", err)
            }
        }
    }
}
