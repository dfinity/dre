use crate::metrics::{DailyMetricsAggregator, NodeDailyMetrics};
use crate::reward_period::{RewardPeriod, TimestampNanos};
use crate::rewards_calculator::RewardsCalculator;
use ic_base_types::{NodeId, PrincipalId};
use itertools::Itertools;
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

mod logs;
mod metrics;
mod reward_period;
mod rewards_calculator;
mod tabled_types;

/// Computes rewards for node providers based on their nodes' performance during the specified `reward_period`.
///
/// Rewards are determined using:
/// - metrics_by_node: Daily node metrics for each node. Only nodes listed in `providers_rewardable_nodes` are considered.
/// - providers_rewardable_nodes: Nodes eligible for rewards, as recorded in the registry versions spanning the `reward_period` provided.
///
/// # Arguments
/// * reward_period - The time frame for which rewards are calculated.
/// * metrics_by_node - A collection of daily node metrics for each node.
/// * providers_rewardable_nodes: A set of nodes eligible for rewards during the `reward_period`.
///
/// TODO: Implement the XDR reward calculation logic from the nodes multiplier.
pub fn calculate_rewards(
    reward_period: RewardPeriod,
    metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
    providers_rewardable_nodes: BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &metrics_by_node, &providers_rewardable_nodes)?;

    let metrics_aggregator = DailyMetricsAggregator {
        metrics_by_node,
        reward_period,
    };
    let failure_rates_by_subnet = metrics_aggregator.calculate_subnet_failure_rates();
    let rewards_calculator = RewardsCalculator::new().with_subnets_failure_rates_discount(failure_rates_by_subnet);

    for (_provider_id, provider_nodes) in providers_rewardable_nodes {
        let provider_nodes_failure_rates = provider_nodes
            .iter()
            .map(|node_id| {
                let failure_rates_in_period = metrics_aggregator.calculate_node_failure_rates_for_period(node_id);
                (*node_id, failure_rates_in_period)
            })
            .collect();

        let _nodes_multiplier = rewards_calculator.calculate_rewards_multipliers(provider_nodes_failure_rates);
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
        // Check if metrics are unique for each day
        let unique_timestamps = metrics_entries.iter().map(|daily_metrics| *daily_metrics.ts).collect::<HashSet<_>>();
        if unique_timestamps.len() != metrics_entries.len() {
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
