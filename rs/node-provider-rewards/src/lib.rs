use crate::multiplier_extrapolation_pipeline::MultiplierExtrapolationPipeline;
use crate::reward_period::{RewardPeriod, UnalignedTimestamp, NANOS_PER_DAY};
use crate::types::{DailyFailureRate, DailyMetrics, FailureRate, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use std::collections::{HashMap, HashSet};
use std::fmt;

mod logs;
mod multiplier_extrapolation_pipeline;
mod reward_period;
mod types;

pub fn calculate_rewards(
    reward_period: RewardPeriod,
    daily_metrics_per_node: HashMap<NodeId, Vec<DailyMetrics>>,
    nodes_per_node_provider: HashMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    validate_input(&reward_period, &daily_metrics_per_node, &nodes_per_node_provider)?;

    let nodes = nodes_per_node_provider.values().flatten().collect::<HashSet<_>>();
    let nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>> = nodes_failure_rates(nodes, &daily_metrics_per_node, &reward_period);
    let multiplier_extrapolation_pipeline = MultiplierExtrapolationPipeline::new(nodes_failure_rates);

    for (_provider_id, provider_nodes) in nodes_per_node_provider {
        let (_nodes_multiplier, _provider_log) = multiplier_extrapolation_pipeline.run(provider_nodes);
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

fn nodes_failure_rates(
    nodes: HashSet<&NodeId>,
    daily_metrics_per_node: &HashMap<NodeId, Vec<DailyMetrics>>,
    reward_period: &RewardPeriod,
) -> HashMap<NodeId, Vec<DailyFailureRate>> {
    let days_in_period = reward_period.days_between();
    nodes
        .into_iter()
        .map(|node_id| {
            let daily_failure_rates: Vec<_> = (0..days_in_period)
                .map(|day| {
                    let ts = reward_period.start_ts + day * NANOS_PER_DAY;
                    let metrics_for_day = daily_metrics_per_node
                        .get(node_id)
                        .and_then(|metrics| metrics.iter().find(|m| UnalignedTimestamp::new(m.ts).align_to_day_start() == ts));
                    let failure_rate = match metrics_for_day {
                        Some(metrics) => FailureRate::Defined {
                            subnet_assigned: metrics.subnet_assigned,
                            value: metrics.failure_rate,
                        },
                        None => FailureRate::Undefined,
                    };

                    DailyFailureRate {
                        ts,
                        failure_rate: failure_rate,
                    }
                })
                .collect();
            (*node_id, daily_failure_rates)
        })
        .collect()
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
