use crate::failure_rates::{FailureRatesInPeriod, NodeDailyFailureRate, SubnetDailyFailureRate};
use crate::nodes_multiplier_calculator::NodesMultiplierCalculator;
use crate::reward_period::{RewardPeriod, NANOS_PER_DAY};
use crate::types::{DailyMetrics, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use std::collections::{HashMap, HashSet};
use std::fmt;

mod failure_rates;
mod logs;
mod nodes_multiplier_calculator;
mod reward_period;
mod types;

pub fn calculate_rewards(
    reward_period: RewardPeriod,
    daily_metrics_per_node: HashMap<NodeId, Vec<DailyMetrics>>,
    nodes_per_node_provider: HashMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &daily_metrics_per_node, &nodes_per_node_provider)?;

    let failure_rates_in_period = FailureRatesInPeriod {
        daily_metrics_per_node,
        reward_period,
    };
    let subnets_failure_rates = failure_rates_in_period.of_all_subnets();

    for (_provider_id, provider_nodes) in nodes_per_node_provider {
        let provider_nodes_failure_rates = failure_rates_in_period.of_nodes(&provider_nodes);
        let _nodes_multiplier = NodesMultiplierCalculator::new(provider_nodes_failure_rates, &subnets_failure_rates).run();
    }

    Ok(())
}

fn validate_input(
    reward_period: &RewardPeriod,
    daily_metrics_per_node: &HashMap<NodeId, Vec<DailyMetrics>>,
    nodes_per_node_provider: &HashMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    let rewardable_nodes: HashSet<&NodeId> = nodes_per_node_provider.values().flatten().collect();
    if rewardable_nodes.is_empty() {
        return Err(RewardCalculationError::EmptyRewardables);
    }

    for metrics in daily_metrics_per_node.values() {
        for metric in metrics {
            if metric.ts % NANOS_PER_DAY != 0 {
                return Err(RewardCalculationError::TimestampNotBeginning(metric.ts));
            }
            if !reward_period.contains(metric.ts) {
                return Err(RewardCalculationError::SubnetMetricsOutOfRange {
                    subnet_id: metric.subnet_assigned,
                    timestamp: metric.ts,
                    reward_period: reward_period.clone(),
                });
            }
        }
    }

    for node_id in daily_metrics_per_node.keys() {
        if !rewardable_nodes.contains(node_id) {
            return Err(RewardCalculationError::NodeNotInRewardables(*node_id));
        }
    }

    Ok(())
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
    TimestampNotBeginning(TimestampNanos),
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
                write!(
                    f,
                    "Node {} has metrics in rewarding period but it is not part of rewardable_nodes",
                    node_id
                )
            }
            RewardCalculationError::TimestampNotBeginning(ts) => {
                write!(f, "No {}", ts)
            }
        }
    }
}
