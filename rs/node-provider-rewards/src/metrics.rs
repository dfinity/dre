use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use ic_base_types::{NodeId, SubnetId};
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
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
            ts: ts.into(),
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
/// - `Extrapolated`: A computed failure rate when data is missing.
/// - `Undefined`: Indicates that no metrics were recorded.
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

pub struct DailyMetricsAggregator {
    pub reward_period: RewardPeriod,
    pub metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
}

impl DailyMetricsAggregator {
    /// Calculates daily failure rates for all subnets within the reward period.
    ///
    /// The failure rate for a subnet on a given day is defined as the **75th percentile**
    /// of the failure rates of all nodes assigned to that subnet. Days with no recorded
    /// metrics for a subnet are omitted.
    pub fn calculate_subnet_failure_rates(&self) -> BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> {
        const PERCENTILE: f64 = 0.75;
        let mut rates_map: BTreeMap<(SubnetId, TimestampNanos), Vec<Decimal>> = BTreeMap::new();

        // Aggregate failure rates by (subnet, timestamp)
        for metrics in self.metrics_by_node.values().flatten() {
            rates_map
                .entry((metrics.subnet_assigned, *metrics.ts))
                .or_default()
                .push(metrics.failure_rate);
        }

        let mut subnet_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> = BTreeMap::new();

        // Compute the 75th percentile for each (subnet, timestamp)
        for ((subnet, ts), mut rates) in rates_map {
            rates.sort();

            let index = ((rates.len() as f64) * PERCENTILE).ceil() as usize - 1;
            let percentile_rate = SubnetDailyFailureRate { ts: ts, value: rates[index] };

            subnet_failure_rates.entry(subnet).or_default().push(percentile_rate);
        }

        subnet_failure_rates
    }

    /// Calculates daily failure rates for a given node over the reward period.
    ///
    /// If a node has no metrics recorded for a day, its failure rate is marked as [NodeFailureRate::Undefined].
    /// Otherwise, it is recorded as [NodeFailureRate::Defined].
    pub fn calculate_node_failure_rates_for_period(&self, node_id: &NodeId) -> Vec<NodeDailyFailureRate> {
        let days_in_period = self.reward_period.days_between();

        (0..days_in_period)
            .map(|day| {
                let ts = TimestampNanosAtDayEnd::from(*self.reward_period.start_ts + day * NANOS_PER_DAY);
                let daily_metrics = self.metrics_by_node.get(node_id).and_then(|metrics| metrics.iter().find(|m| m.ts == ts));

                let failure_status = match daily_metrics {
                    Some(metrics) => NodeFailureRate::Defined {
                        subnet_assigned: metrics.subnet_assigned,
                        value: metrics.failure_rate,
                    },
                    None => NodeFailureRate::Undefined,
                };
                NodeDailyFailureRate {
                    ts: *ts,
                    value: failure_status,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
