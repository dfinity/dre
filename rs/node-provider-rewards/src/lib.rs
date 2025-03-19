use crate::execution_context::{nodes_ids, ExecutionContext, RewardsCalculationResult, XDRPermyriad};
use crate::metrics::{nodes_failure_rates_in_period, subnets_failure_rates, NodeDailyMetrics};
use crate::reward_period::{RewardPeriod, TimestampNanos};
use crate::types::{rewardable_nodes_by_provider, RewardableNode};
use ::tabled::Table;
use ic_base_types::{NodeId, PrincipalId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use num_traits::ToPrimitive;
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt;

mod execution_context;
mod metrics;
mod reward_period;
mod tabled;
mod types;

pub struct RewardsPerNodeProvider {
    pub rewards_per_provider: BTreeMap<PrincipalId, XDRPermyriad>,
    pub computation_table_per_provider: BTreeMap<PrincipalId, Vec<Table>>,
}

/// Computes rewards for node providers based on their nodes' performance during the specified `reward_period`.
///
/// # Arguments
/// * reward_period - The time frame for which rewards are calculated.
/// * rewards_table - The rewards table containing the reward rates for each node type.
/// * metrics_by_node - Daily node metrics for nodes in `reward_period`. Only nodes in `providers_rewardable_nodes` keys are considered.
/// * rewardable_nodes: Nodes eligible for rewards, as recorded in the registry versions spanning the `reward_period` provided.
pub fn calculate_rewards(
    reward_period: &RewardPeriod,
    rewards_table: &NodeRewardsTable,
    metrics_by_node: &BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
    rewardable_nodes: &[RewardableNode],
) -> Result<RewardsPerNodeProvider, RewardCalculationError> {
    let mut rewards_per_provider = BTreeMap::new();
    let mut computation_table_per_provider = BTreeMap::new();
    let all_nodes = nodes_ids(rewardable_nodes);

    validate_input(reward_period, metrics_by_node, &all_nodes)?;

    let ctx = ExecutionContext::new(
        nodes_failure_rates_in_period(&all_nodes, reward_period, metrics_by_node),
        subnets_failure_rates(metrics_by_node),
        rewards_table.clone(),
    );

    for (provider_id, provider_nodes) in rewardable_nodes_by_provider(rewardable_nodes) {
        let RewardsCalculationResult {
            rewards,
            computation_log_tabled,
        } = ctx.calculate_rewards(provider_nodes);

        rewards_per_provider.insert(provider_id, rewards.to_u64().expect("Conversion succeeded"));
        computation_table_per_provider.insert(provider_id, computation_log_tabled);
    }

    Ok(RewardsPerNodeProvider {
        rewards_per_provider,
        computation_table_per_provider,
    })
}

fn validate_input(
    reward_period: &RewardPeriod,
    metrics_by_node: &BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
    all_nodes: &[NodeId],
) -> Result<(), RewardCalculationError> {
    if all_nodes.is_empty() {
        return Err(RewardCalculationError::EmptyNodes);
    }

    // Check if all nodes with metrics are present in the rewardable nodes
    for node_id in metrics_by_node.keys() {
        if !all_nodes.contains(node_id) {
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

#[cfg(test)]
mod tests;
