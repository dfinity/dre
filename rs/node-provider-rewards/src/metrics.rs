use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use ic_base_types::{NodeId, SubnetId};
use itertools::Itertools;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashSet};
use std::fmt;

/// Represents the daily metrics recorded for a node.
#[derive(Clone, PartialEq, Debug)]
pub struct NodeDailyMetrics {
    pub ts: TimestampNanosAtDayEnd,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

impl fmt::Display for NodeDailyMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "blocks_proposed: {}, blocks_failed: {}, failure_rate: {}",
            self.num_blocks_proposed, self.num_blocks_failed, self.failure_rate
        )
    }
}

impl NodeDailyMetrics {
    /// Constructs a new set of daily metrics for a node.
    pub fn new(ts: TimestampNanos, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let total_blocks = num_blocks_proposed + num_blocks_failed;
        let failure_rate = if total_blocks == 0 {
            Decimal::ZERO
        } else {
            Decimal::from_f64(num_blocks_failed as f64 / total_blocks as f64).unwrap()
        };
        NodeDailyMetrics {
            ts: TimestampNanosAtDayEnd::from(ts),
            num_blocks_proposed,
            num_blocks_failed,
            subnet_assigned,
            failure_rate,
        }
    }
}

/// Represents the failure rate status of a node for a day.
///
/// The failure rate is a `Decimal` in the range [0, 1]. The variants provide explicit meaning:
/// - `Defined`: A recorded failure rate for a node assigned to a subnet.
/// - `DefinedRelative`: A failure rate adjusted by the subnet’s failure rate.
/// - `Extrapolated`: An extrapolated failure rate used when `Undefined` failure rates are extrapolated.
/// - `Undefined`: Indicates that no metrics were recorded because the node is not assigned to any subnet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeFailureRate {
    Defined {
        subnet_assigned: SubnetId,
        value: Decimal,
    },
    DefinedRelative {
        subnet_assigned: SubnetId,
        original_failure_rate: Decimal,
        subnet_failure_rate: Decimal,
        value: Decimal,
    },
    Extrapolated(Decimal),
    Undefined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDailyFailureRate {
    pub ts: TimestampNanos,
    pub value: NodeFailureRate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubnetDailyFailureRate {
    pub ts: TimestampNanos,
    pub value: Decimal,
}

/// The percentile used to calculate the failure rate for a subnet.
const SUBNET_FAILURE_RATE_PERCENTILE: f64 = 0.75;

/// Computes the failure rates for each subnet on a given day.
///
/// The failure rate for a subnet on a given day is defined as the `SUBNET_FAILURE_RATE_PERCENTILE`
/// of the failure rates of all nodes assigned to that subnet. Days with no recorded
/// metrics for a subnet are omitted.
pub fn subnets_failure_rates(metrics_by_node: &BTreeMap<NodeId, Vec<NodeDailyMetrics>>) -> BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> {
    let mut rates_map: BTreeMap<(SubnetId, TimestampNanos), Vec<Decimal>> = BTreeMap::new();

    // Aggregate failure rates by (subnet, timestamp)
    for metrics in metrics_by_node.values().flatten() {
        rates_map
            .entry((metrics.subnet_assigned, *metrics.ts))
            .or_default()
            .push(metrics.failure_rate);
    }

    let mut subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> = BTreeMap::new();

    for ((subnet, ts), mut rates) in rates_map {
        rates.sort();

        let index = ((rates.len() as f64) * SUBNET_FAILURE_RATE_PERCENTILE).ceil() as usize - 1;
        let percentile_rate = SubnetDailyFailureRate { ts, value: rates[index] };

        subnets_failure_rates.entry(subnet).or_default().push(percentile_rate);
    }

    subnets_failure_rates
}

/// Calculates daily failure rates for a given set of `nodes` over the reward period.
///
/// If a node has no metrics recorded for a day, its failure rate is marked as [NodeFailureRate::Undefined].
/// Otherwise, it is recorded as [NodeFailureRate::Defined].
/// `metrics_by_node.keys()` must be a subset of `nodes`.
pub fn nodes_failure_rates_in_period(
    nodes: &[NodeId],
    reward_period: &RewardPeriod,
    metrics_by_node: &BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
) -> BTreeMap<NodeId, Vec<NodeDailyFailureRate>> {
    let days_in_period = reward_period.days_between();

    nodes
        .iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|node_id| {
            let failure_rates_in_period = (0..days_in_period)
                .map(|day| {
                    let ts = TimestampNanosAtDayEnd::from(*reward_period.start_ts + day * NANOS_PER_DAY);

                    let value = match metrics_by_node.get(node_id) {
                        Some(metrics) => {
                            let metrics_for_day = metrics.iter().filter(|m| m.ts == ts).collect_vec();
                            node_failure_rate(metrics_for_day)
                        }
                        None => NodeFailureRate::Undefined,
                    };
                    NodeDailyFailureRate { ts: *ts, value }
                })
                .collect_vec();

            (*node_id, failure_rates_in_period)
        })
        .collect()
}

fn node_failure_rate(one_day_metrics: Vec<&NodeDailyMetrics>) -> NodeFailureRate {
    // Node is assigned in reward period but has no metrics for the day.
    if one_day_metrics.is_empty() {
        return NodeFailureRate::Undefined;
    };

    // Node is assigned to only one subnet.
    if one_day_metrics.len() == 1 {
        let first = one_day_metrics.first().expect("No metrics");

        return NodeFailureRate::Defined {
            subnet_assigned: first.subnet_assigned,
            value: first.failure_rate,
        };
    }

    // Node is reassigned to different subnets on the same day.
    // The algorithm considers for this case the subnet where the node has proposed more blocks.
    let mut subnet_block_counts: BTreeMap<SubnetId, u64> = BTreeMap::new();

    for metrics in one_day_metrics.iter() {
        *subnet_block_counts.entry(metrics.subnet_assigned).or_insert(0) += metrics.num_blocks_proposed;
    }

    let (subnet_assigned, _) = subnet_block_counts.into_iter().max_by_key(|&(_, count)| count).expect("No subnet found");

    let failure_rate = one_day_metrics
        .iter()
        .find(|m| m.subnet_assigned == subnet_assigned)
        .expect("No metrics for the selected subnet")
        .failure_rate;

    NodeFailureRate::Defined {
        subnet_assigned,
        value: failure_rate,
    }
}
