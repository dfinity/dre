use crate::metrics::{nodes_failure_rates_in_period, subnets_failure_rates, NodeDailyMetrics};
use crate::performance_calculator::PerformanceMultiplierCalculator;
use crate::reward_period::{RewardPeriod, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId};
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

mod logs;
mod metrics;
mod performance_calculator;
mod reward_period;
mod tabled_types;
#[cfg(test)]
mod tests;

/// Computes rewards for node providers based on their nodes' performance during the specified `reward_period`.
///
/// # Arguments
/// * reward_period - The time frame for which rewards are calculated.
/// * metrics_by_node - Daily node metrics for each node in `reward_period`. Only nodes listed in `providers_rewardable_nodes` are considered.
/// * providers_rewardable_nodes: Nodes eligible for rewards, as recorded in the registry versions spanning the `reward_period` provided.
///
/// TODO: Implement the XDR reward calculation logic from the nodes multiplier.
pub fn calculate_rewards(
    reward_period: RewardPeriod,
    metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
    providers_rewardable_nodes: BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &metrics_by_node, &providers_rewardable_nodes)?;

    let subnets_failure_rates = subnets_failure_rates(&metrics_by_node);
    let all_nodes: Vec<NodeId> = providers_rewardable_nodes.values().flatten().cloned().collect();
    let nodes_failure_rates = nodes_failure_rates_in_period(&all_nodes, &reward_period, &metrics_by_node);

    let perf_calculator = PerformanceMultiplierCalculator::new(nodes_failure_rates, subnets_failure_rates);

    for (_provider_id, provider_nodes) in providers_rewardable_nodes {
        let _nodes_multiplier = perf_calculator.calculate_performance_multipliers(&provider_nodes);
    }

    Ok(())
}

fn validate_input(
    reward_period: &RewardPeriod,
    metrics_by_node: &BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
    providers_rewardable_nodes: &BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    let rewardable_nodes: HashSet<&NodeId> = providers_rewardable_nodes.values().flatten().collect();
    if rewardable_nodes.is_empty() {
        return Err(RewardCalculationError::EmptyNodes);
    }

    // Check if all nodes with metrics are present in the rewardable nodes
    for node_id in metrics_by_node.keys() {
        if !rewardable_nodes.contains(node_id) {
            return Err(RewardCalculationError::NodeNotInRewardables(*node_id));
        }
    }

    for (node_id, metrics_entries) in metrics_by_node {
        for entry in metrics_entries {
            // Check if all metrics are within the reward period
            if !reward_period.contains(*entry.ts) {
                return Err(RewardCalculationError::NodeMetricsOutOfRange {
                    node_id: *node_id,
                    timestamp: *entry.ts,
                    reward_period: reward_period.clone(),
                });
            }
        }
        // Metrics are unique if there are no duplicate entries for the same day and subnet.
        // Metrics with the same timestamp and different subnet are allowed.
        let unique_timestamp_subnet = metrics_entries
            .iter()
            .map(|daily_metrics| (*daily_metrics.ts, daily_metrics.subnet_assigned))
            .collect::<HashSet<_>>();
        if unique_timestamp_subnet.len() != metrics_entries.len() {
            return Err(RewardCalculationError::DuplicateMetrics(*node_id));
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum RewardCalculationError {
    EmptyNodes,
    NodeNotInRewardables(NodeId),
    NodeMetricsOutOfRange {
        node_id: NodeId,
        timestamp: TimestampNanos,
        reward_period: RewardPeriod,
    },
    DuplicateMetrics(NodeId),
}

impl Error for RewardCalculationError {}

impl fmt::Display for RewardCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RewardCalculationError::EmptyNodes => {
                write!(f, "No rewardable nodes provided")
            }
            RewardCalculationError::NodeNotInRewardables(node_id) => {
                write!(f, "Node {} has metrics but it is not part of rewardable nodes", node_id)
            }
            RewardCalculationError::NodeMetricsOutOfRange {
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
            RewardCalculationError::DuplicateMetrics(node_id) => {
                write!(f, "Node {} has multiple metrics for the same day", node_id)
            }
        }
    }
}
