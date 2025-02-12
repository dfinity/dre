use crate::logs::NodeProviderRewardsLog;
use crate::reward_period::RewardPeriod;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_management_canister_types::NodeMetricsHistoryResponse;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fmt;
use chrono::NaiveDate;

pub type RegionNodeTypeCategory = (String, String);
pub type TimestampNanos = u64;
pub type SubnetMetricsHistory = (PrincipalId, Vec<NodeMetricsHistoryResponse>);

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct NodeRewardsMultiplier {
    pub node_id: NodeId,
    pub multiplier: Decimal,
}

pub struct NodeRewardsMultiplierResult {
    pub log_per_node_provider: HashMap<PrincipalId, NodeProviderRewardsLog>,
    pub nodes_multiplier: Vec<NodeRewardsMultiplier>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct DailyMetrics {
    pub ts: u64,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

#[derive(Clone)]
pub struct SystematicFailureRate(pub Decimal);
impl SystematicFailureRate {
    const PERCENTILE: f64 = 0.75;

    pub fn from_failure_rates(failure_rates: Vec<Decimal>) -> Self {
        let mut failure_rates = failure_rates;
        failure_rates.sort();

        let len = failure_rates.len();
        if len == 0 {
            return Self(Decimal::ZERO);
        }
        let idx = ((len as f64) * Self::PERCENTILE).ceil() as usize - 1;

        Self(failure_rates[idx])
    }

    pub fn get_relative_failure_rate(&self, failure_rate: Decimal) -> Decimal {
        let relative_failure_rate = if failure_rate < self.0 {
            Decimal::ZERO
        } else {
            failure_rate - self.0
        };

        relative_failure_rate
    }
}

impl Default for DailyMetrics {
    fn default() -> Self {
        DailyMetrics {
            ts: 0,
            subnet_assigned: SubnetId::from(PrincipalId::new_anonymous()),
            num_blocks_proposed: 0,
            num_blocks_failed: 0,
            failure_rate: Decimal::ZERO,
        }
    }
}
impl fmt::Display for DailyMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "num_blocks_proposed: {},  num_blocks_failed: {}, failure_rate: {}",
            self.num_blocks_proposed, self.num_blocks_failed, self.failure_rate
        )
    }
}

impl DailyMetrics {
    pub fn new(
        ts: u64,
        subnet_assigned: SubnetId,
        num_blocks_proposed: u64,
        num_blocks_failed: u64,
    ) -> Self {
        let daily_total = num_blocks_proposed + num_blocks_failed;
        let failure_rate = if daily_total == 0 {
            Decimal::ZERO
        } else {
            Decimal::from_f64(num_blocks_failed as f64 / daily_total as f64).unwrap()
        };
        DailyMetrics {
            ts,
            num_blocks_proposed,
            num_blocks_failed,
            subnet_assigned,
            failure_rate,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RewardCalculationError {
    SubnetMetricsOutOfRange {
        subnet_id: SubnetId,
        timestamp: TimestampNanos,
        reward_period: RewardPeriod,
    },
    EmptyRewardables,
    NodeNotInRewardables(NodeId),
}

impl fmt::Display for RewardCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardCalculationError::SubnetMetricsOutOfRange {
                subnet_id,
                timestamp,
                reward_period,
            } => {
                write!(
                    f,
                    "Subnet {} has metrics outside the reward period: timestamp: {} not in {}",
                    subnet_id, timestamp, reward_period
                )
            }
            RewardCalculationError::EmptyRewardables => {
                write!(f, "No rewardable nodes were provided")
            }
            RewardCalculationError::NodeNotInRewardables(node_id) => {
                write!(f, "Node {} has metrics in rewarding period but it is not part of rewardable_nodes", node_id)
            }
        }
    }
}
