use crate::metrics::{DailyMetricsProcessor, DailyNodeMetrics};
use crate::nodes_multiplier_calculator::RewardsMultiplierCalculator;
use crate::reward_period::{RewardPeriod, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId};
use itertools::Itertools;
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

mod logs;
mod metrics;
mod nodes_multiplier_calculator;
mod reward_period;
mod tabled_types;

/// Computes rewards for node providers based on their nodes' performance during the specified `reward_period`.
///
/// Rewards are determined using:
/// - daily_metrics_per_node: Daily node metrics for each node. Only nodes listed in `providers_rewardable_nodes` are considered.
/// - providers_rewardable_nodes: Nodes eligible for rewards, as recorded in the registry versions spanning the `reward_period` provided.
///
/// # Arguments
/// * reward_period - The time frame for which rewards are calculated.
/// * daily_metrics_per_node - A collection of daily node metrics for each node.
/// * providers_rewardable_nodes: A set of nodes eligible for rewards during the `reward_period`.
///
/// TODO: Implement the XDR reward calculation logic from the nodes multiplier.
pub fn calculate_rewards(
    reward_period: RewardPeriod,
    daily_metrics_per_node: BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
    providers_rewardable_nodes: BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &daily_metrics_per_node, &providers_rewardable_nodes)?;

    let daily_metrics_processor = DailyMetricsProcessor {
        daily_metrics_per_node,
        reward_period,
    };
    let subnets_failure_rates = daily_metrics_processor.daily_failure_rates_per_subnet();
    let multiplier_calculator = RewardsMultiplierCalculator::new().with_subnets_failure_rate_discount(subnets_failure_rates);

    for (_provider_id, provider_nodes) in providers_rewardable_nodes {
        let provider_nodes_failure_rates = provider_nodes
            .iter()
            .map(|node_id| {
                let failure_rates_in_period = daily_metrics_processor.daily_failure_rates_in_period(node_id);
                (*node_id, failure_rates_in_period)
            })
            .collect();

        let _nodes_multiplier = multiplier_calculator.rewards_multiplier_per_node(provider_nodes_failure_rates);
    }

    Ok(())
}

fn validate_input(
    reward_period: &RewardPeriod,
    daily_metrics_per_node: &BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
    providers_rewardable_nodes: &BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    let rewardable_nodes: HashSet<&NodeId> = providers_rewardable_nodes.values().flatten().collect();
    if rewardable_nodes.is_empty() {
        return Err(RewardCalculationError::EmptyNodes);
    }

    // Check if all nodes with metrics are present in the rewardable nodes
    for node_id in daily_metrics_per_node.keys() {
        if !rewardable_nodes.contains(node_id) {
            return Err(RewardCalculationError::NodeNotInRewardables(*node_id));
        }
    }

    for (node_id, all_daily_metrics) in daily_metrics_per_node {
        for daily_metrics in all_daily_metrics {
            // Check if all metrics are within the reward period
            if !reward_period.contains(*daily_metrics.ts) {
                return Err(RewardCalculationError::NodeMetricsOutOfRange {
                    node_id: *node_id,
                    timestamp: *daily_metrics.ts,
                    reward_period: reward_period.clone(),
                });
            }
        }
        // Check if metrics are unique for each day
        let unique_timestamps = all_daily_metrics.iter().map(|daily_metrics| *daily_metrics.ts).collect::<HashSet<_>>();
        if unique_timestamps.len() != all_daily_metrics.len() {
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
