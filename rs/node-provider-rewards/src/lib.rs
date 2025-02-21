use crate::metrics::{DailyNodeMetrics, MetricsProcessor};
use crate::nodes_multiplier_calculator::NodesMultiplierCalculator;
use crate::reward_period::{RewardPeriod, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId};
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

mod logs;
mod metrics;
mod nodes_multiplier_calculator;
mod reward_period;

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
/// # Errors
/// - RewardCalculationError::EmptyNodes – No rewardable nodes were provided.
/// - RewardCalculationError::NodeNotInRewardables – A node has recorded metrics but is not listed as rewardable.
/// - RewardCalculationError::NodeMetricsOutOfRange – A node has metrics that fall outside the reward_period.
///
/// TODO: Implement the XDR reward calculation logic from the nodes multiplier.
pub fn calculate_rewards(
    reward_period: RewardPeriod,
    daily_metrics_per_node: BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
    providers_rewardable_nodes: BTreeMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &daily_metrics_per_node, &providers_rewardable_nodes)?;

    let metrics_processor = MetricsProcessor {
        daily_metrics_per_node,
        reward_period,
    };
    let subnets_failure_rates = metrics_processor.daily_failure_rates_per_subnet();
    let multiplier_calculator = NodesMultiplierCalculator::new().with_subnets_fr_discount(subnets_failure_rates);

    for (_provider_id, provider_nodes) in providers_rewardable_nodes {
        let provider_nodes_failure_rates = provider_nodes
            .iter()
            .map(|node_id| {
                let failure_rates_in_period = metrics_processor.daily_failure_rates_in_period(node_id);
                (*node_id, failure_rates_in_period)
            })
            .collect();

        let _nodes_multiplier = multiplier_calculator.run(provider_nodes_failure_rates);
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

    // Check if all metrics are within the reward period
    for (node_id, all_daily_metrics) in daily_metrics_per_node {
        for daily_metrics in all_daily_metrics {
            if !reward_period.contains(daily_metrics.ts) {
                return Err(RewardCalculationError::NodeMetricsOutOfRange {
                    node_id: *node_id,
                    timestamp: daily_metrics.ts,
                    reward_period: reward_period.clone(),
                });
            }
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
        }
    }
}
