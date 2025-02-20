use crate::metrics::{DailyNodeMetrics, MetricsProcessor};
use crate::nodes_multiplier_calculator::NodesMultiplierCalculator;
use crate::reward_period::{RewardPeriod, TimestampNanos, NANOS_PER_DAY};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

mod logs;
mod metrics;
mod nodes_multiplier_calculator;
mod reward_period;

pub fn calculate_rewards(
    reward_period: RewardPeriod,
    daily_metrics_per_node: BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
    nodes_per_provider: BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &daily_metrics_per_node, &nodes_per_provider)?;

    let metrics_processor = MetricsProcessor {
        daily_metrics_per_node,
        reward_period,
    };
    let subnets_failure_rates = metrics_processor.daily_failure_rates_per_subnet();
    let multiplier_calculator = NodesMultiplierCalculator::new().with_subnets_fr_discount(subnets_failure_rates);

    for (_provider_id, provider_nodes) in nodes_per_provider {
        let provider_nodes_failure_rates = metrics_processor.daily_failure_rates_per_node(&provider_nodes);
        let _nodes_multiplier = multiplier_calculator.run(provider_nodes_failure_rates);
    }

    Ok(())
}

fn validate_input(
    reward_period: &RewardPeriod,
    daily_metrics_per_node: &BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
    nodes_per_provider: &BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    let nodes: HashSet<&NodeId> = nodes_per_provider.values().flatten().collect();
    if nodes.is_empty() {
        return Err(RewardCalculationError::EmptyNodes);
    }

    for metrics in daily_metrics_per_node.values() {
        for metric in metrics {
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
        if !nodes.contains(node_id) {
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
    EmptyNodes,
    NodeNotInRewardables(NodeId),
}

impl Error for RewardCalculationError {}

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
            RewardCalculationError::EmptyNodes => {
                write!(f, "No rewardable nodes were provided")
            }
            RewardCalculationError::NodeNotInRewardables(node_id) => {
                write!(
                    f,
                    "Node {} has metrics in rewarding period but it is not part of rewardable_nodes",
                    node_id
                )
            }
        }
    }
}
