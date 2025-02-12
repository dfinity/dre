use crate::types::{DailyFailureRatesData, SystematicFailureRate};
use crate::logs::{LogEntry, Operation, NodeProviderRewardsLog, DailyFailureRatesData};
use crate::reward_period::RewardPeriod;
use crate::types::{
    DailyMetrics, NodeRewardsMultiplier, NodeRewardsMultiplierResult, RewardCalculationError,
    TimestampNanos,
};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[macro_export]
macro_rules! log_entry {
    ($entry:expr) => {
        provider_log.add_entry($entry)
    };
}


const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);
const RF: &str = "Linear Reduction factor";

pub fn node_rewards_multiplier(
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
    let days_in_period = reward_period.days_between();

    // Precompute systematic failure rates for each subnet/day combination
    let systematic_rates = compute_systematic_failure_rates(&daily_metrics_per_node);

    for (provider_id, provider_nodes) in nodes_per_node_provider {
        let provider_log = node_provider_logs.entry(provider_id).or_default();
        let provider_metrics: HashMap<_, _> = daily_metrics_per_node
            .iter()
            .filter(|(node_id, _)| provider_nodes.contains(node_id))
            .map(|(node_id, metrics)| (*node_id, metrics.clone()))
            .collect();

        // Calculate relative failure rates discounting subnet performance
        let mut node_failure_rates = compute_relative_failure_rates(
            provider_log,
            provider_metrics,
            &systematic_rates,
        );

        // Extrapolate failure rate for days when nodes were unassigned
        let failure_rate_extrapolated = extrapolate_failure_rate(
            provider_log,
            &node_failure_rates,
        );

        // Calculate multiplier for nodes that were completely unassigned the entire period
        let unassigned_multiplier = compute_unassigned_multiplier(
            provider_log,
            &failure_rate_extrapolated,
        );

        // Calculate multipliers for nodes with assignments during the period
        let mut assigned_multipliers = compute_assigned_multipliers(
            provider_log,
            &mut node_failure_rates,
            days_in_period,
            failure_rate_extrapolated,
        );

        // Aggregate results for all provider nodes
        for node_id in provider_nodes {
            let multiplier = assigned_multipliers.remove(&node_id)
                .unwrap_or_else(|| {
                    log_entry!(LogEntry::NodeStatusUnassigned);
                    unassigned_multiplier
                });

            nodes_multipliers.push(NodeRewardsMultiplier {
                node_id,
                multiplier,
            });
        }
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

    for (subnet_id, metrics) in daily_metrics_per_node {
        for metric in metrics {
            if !reward_period.contains(metric.ts) {
                return Err(RewardCalculationError::SubnetMetricsOutOfRange {
                    subnet_id: *subnet_id,
                    timestamp: metric.timestamp_nanos,
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


/// Computes the systematic failure rate for each subnet per day.
///
/// This function calculates the 75th percentile of failure rates for each subnet on a daily basis.
/// This represents the systematic failure rate for all the nodes in the subnet for that day.
fn compute_systematic_failure_rates(
    daily_node_metrics: &HashMap<NodeId, Vec<DailyMetrics>>,
) -> HashMap<(SubnetId, TimestampNanos), SystematicFailureRate> {
    daily_node_metrics
        .values()
        .flatten()
        .chunk_by(|metric| (metric.subnet_assigned, metric.ts))
        .into_iter()
        .map(|(key, group)| {
            let failure_rates: Vec<Decimal> = group.map(|m| m.failure_rate).collect();

            (
                key,
                SystematicFailureRate::from_failure_rates(failure_rates),
            )
        })
        .collect()
}

/// Computes the daily relative node failure rates in the period
fn compute_relative_failure_rates(
    logger: &mut NodeProviderRewardsLog,
    daily_metrics: HashMap<NodeId, Vec<DailyMetrics>>,
    systematic_failure_rate_per_subnet: &HashMap<(SubnetId, TimestampNanos), SystematicFailureRate>,
) -> HashMap<NodeId, Vec<Decimal>> {
    daily_metrics
        .into_iter()
        .map(|(node_id, metrics)| {
            let relative_failure_rates = metrics
                .into_iter()
                .map(|metric| {
                    let systematic_fr = systematic_failure_rate_per_subnet
                        .get(&(metric.subnet_assigned, metric.ts))
                        .cloned()
                        .expect("Systematic failure rate not found");

                    let relative_fr = systematic_fr.get_relative_failure_rate(metric.failure_rate);

                    relative_fr
                })
                .collect();
            (node_id, relative_failure_rates)
        })
        .collect()
}

fn extrapolate_failure_rate(
    logger: &mut NodeProviderRewardsLog,
    node_failure_rates: &HashMap<NodeId, Vec<Decimal>>,
) -> Decimal {
    logger.add_entry(LogEntry::ComputeFailureRateUnassignedDays);
    if node_failure_rates.is_empty() {
        return logger.execute("No nodes assigned", Operation::Set(dec!(1)));
    }

    let avg_rates: Vec<Decimal> = node_failure_rates
        .values()
        .map(|rates| logger.execute("Average failure rate", Operation::Avg(rates.clone())))
        .collect();

    logger.execute("Unassigned days failure rate:", Operation::Avg(avg_rates))
}

fn compute_assigned_multipliers(
    logger: &mut NodeProviderRewardsLog,
    failure_rates: &mut HashMap<NodeId, Vec<Decimal>>,
    days_in_period: u64,
    default_rate: Decimal,
) -> HashMap<NodeId, Decimal> {
    failure_rates
        .iter_mut()
        .map(|(node_id, rates)| {
            logger.add_entry(LogEntry::ComputeNodeMultiplier(*node_id));
            rates.resize(days_in_period as usize, default_rate);

            let avg_rate = logger.execute("Failure rate average", Operation::Avg(rates.clone()));

            let reduction = rewards_reduction_percent(logger, &avg_rate);
            let multiplier =
                logger.execute("Reward Multiplier", Operation::Subtract(dec!(1), reduction));

            (*node_id, multiplier)
        })
        .collect()
}

fn compute_unassigned_multiplier(logger: &mut NodeProviderRewardsLog, failure_rate: &Decimal) -> Decimal {
    let reduction = rewards_reduction_percent(logger, failure_rate);
    logger.execute(
        "Reward multiplier fully unassigned nodes:",
        Operation::Subtract(dec!(1), reduction),
    )
}
/// Calculates the rewards reduction based on the failure rate.
///
/// if `failure_rate` is:
/// - Below the `MIN_FAILURE_RATE`, no reduction in rewards applied.
/// - Above the `MAX_FAILURE_RATE`, maximum reduction in rewards applied.
/// - Within the defined range (`MIN_FAILURE_RATE` to `MAX_FAILURE_RATE`),
///   the function calculates the reduction from the linear reduction function.
fn rewards_reduction_percent(logger: &mut NodeProviderRewardsLog, failure_rate: &Decimal) -> Decimal {
    if failure_rate < &MIN_FAILURE_RATE {
        logger.execute(
            &format!(
                "No Reduction applied because {} is less than {} failure rate.\n{}",
                failure_rate.round_dp(4),
                MIN_FAILURE_RATE,
                RF
            ),
            Operation::Set(dec!(0)),
        )
    } else if failure_rate > &MAX_FAILURE_RATE {
        logger.execute(
            &format!(
                "Max reduction applied because {} is over {} failure rate.\n{}",
                failure_rate.round_dp(4),
                MAX_FAILURE_RATE,
                RF
            ),
            Operation::Set(dec!(0.8)),
        )
    } else {
        let rewards_reduction = (*failure_rate - MIN_FAILURE_RATE)
            / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)
            * MAX_REWARDS_REDUCTION;
        logger.add_entry(LogEntry::RewardsReductionPercent {
            failure_rate: *failure_rate,
            min_fr: MIN_FAILURE_RATE,
            max_fr: MAX_FAILURE_RATE,
            max_rr: MAX_REWARDS_REDUCTION,
            rewards_reduction,
        });

        rewards_reduction
    }
}
