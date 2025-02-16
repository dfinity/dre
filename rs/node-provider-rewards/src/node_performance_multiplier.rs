use crate::logs::{OperationCalculator, LogEntry, Logger, Operation};
use crate::reward_period::{RewardPeriod, UnalignedTimestamp, NANOS_PER_DAY};
use crate::types::{DailyFailureRate, DailyMetrics, FailureRate, NodeRewardsMultiplier, NodeRewardsMultiplierResult, TimestampNanos};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{HashMap, HashSet};
use std::fmt;
use tabular::{Row, Table};
use crate::multiplier_extrapolation_pipeline::MultiplierExtrapolationPipeline;

pub fn node_performance_multiplier(
    reward_period: RewardPeriod,
    daily_metrics_per_node: HashMap<NodeId, Vec<DailyMetrics>>,
    nodes_per_node_provider: HashMap<PrincipalId, Vec<NodeId>>,
) -> Result<NodeRewardsMultiplierResult, RewardCalculationError> {
    validate_input(
        &reward_period,
        &daily_metrics_per_node,
        &nodes_per_node_provider,
    )?;
    let mut node_provider_logs = HashMap::new();
    let mut nodes_multipliers = Vec::new();

    let nodes = nodes_per_node_provider.values().flatten().collect::<HashSet<_>>();
    let mut nodes_failure_rates: HashMap<NodeId, Vec<DailyFailureRate>> = nodes_failure_rates(
        nodes,
        &daily_metrics_per_node,
        &reward_period
    );
    // `subnets_failure_rate` will be later discounted from the node failure rates
    let subnets_failure_rate = compute_subnets_failure_rate(&nodes_failure_rates);

    for (provider_id, provider_nodes) in nodes_per_node_provider {
        let provider_nodes_failure_rates = provider_nodes
            .iter()
            .map(|node_id| {
                (*node_id, nodes_failure_rates.remove(node_id).expect("Node failure rates not found"))
            })
            .collect::<HashMap<_, _>>();

        let pipeline = MultiplierExtrapolationPipeline::new(
            provider_nodes_failure_rates,
            &subnets_failure_rate,
        );

        let (nodes_multiplier, provider_log) = pipeline.run();
        node_provider_logs.insert(provider_id, provider_log);
        nodes_multipliers.extend(nodes_multiplier);
    }

    Ok(NodeRewardsMultiplierResult {
        log_per_node_provider: node_provider_logs,
        nodes_multiplier: nodes_multipliers,
    })

}

fn validate_input(
    reward_period: &RewardPeriod,
    daily_metrics_per_node: &HashMap<NodeId, Vec<DailyMetrics>>,
    nodes_per_node_provider: &HashMap<PrincipalId, Vec<NodeId>>,
) -> Result<(), RewardCalculationError> {
    let rewardable_nodes: HashSet<&NodeId> = nodes_per_node_provider
        .values()
        .flatten()
        .collect();
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
        .iter()
        .map(|node_id| {
            let daily_failure_rates = (0..days_in_period)
                .map(|day| {
                    let ts = reward_period.start_ts + day * NANOS_PER_DAY;
                    let metrics_for_day = daily_metrics_per_node
                        .get(node_id)
                        .and_then(|metrics| metrics.iter().find(|m| {
                            UnalignedTimestamp::new(m.ts).align_to_day_start() == ts
                        }));
                    let failure_rate = match metrics_for_day {
                        Some(metrics) => FailureRate::Defined {
                            subnet_assigned: metrics.subnet_assigned,
                            value: metrics.failure_rate,
                        },
                        None => FailureRate::Undefined,
                    };

                    DailyFailureRate {
                        ts,
                        value: failure_rate,
                    }
                })
                .collect();
            (node_id, daily_failure_rates)
        })
        .collect()
}

/// Computes the systematic failure rate for each subnet per day.
///
/// This function calculates the 75th percentile of failure rates for each subnet on a daily basis.
/// This represents the systematic failure rate for all the nodes in the subnet for that day.
fn compute_subnets_failure_rate(
    nodes_failure_rates: &HashMap<NodeId, Vec<DailyFailureRate>>,
) -> HashMap<(SubnetId, TimestampNanos), Decimal> {

    const PERCENTILE: f64 = 0.75;

    fn from_failure_rates(failure_rates: Vec<Decimal>) -> Decimal {
        let mut failure_rates = failure_rates;
        failure_rates.sort();

        let len = failure_rates.len();
        if len == 0 {
            return Decimal::ZERO;
        }
        let idx = ((len as f64) * PERCENTILE).ceil() as usize - 1;

        failure_rates[idx]
    }

    nodes_failure_rates
        .values()
        .flatten()
        .filter_map(|metric| {
            match metric.value {
                FailureRate::Defined { subnet_assigned, value } => Some((subnet_assigned, metric.ts, value)),
                _ => None,
            }
        })
        .chunk_by(|(subnet_assigned, ts, _)| (subnet_assigned, ts))
        .into_iter()
        .map(|(key, group)| {
            let failure_rates: Vec<Decimal> = group.map(|(_, _, failure_rate)| failure_rate).collect();

            (
                key,
                from_failure_rates(failure_rates),
            )
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
                write!(f, "Node {} has metrics in rewarding period but it is not part of rewardable_nodes", node_id)
            }
            RewardCalculationError::TimestampNotBeginning(ts) => {
                write!(f, "No {}", ts)
            }
        }
    }
}
#[cfg(test)]
mod tests;
