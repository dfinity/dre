use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use ic_base_types::{NodeId, SubnetId};
use itertools::Itertools;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, PartialEq, Debug)]
pub struct DailyNodeMetrics {
    pub ts: TimestampNanosAtDayEnd,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

impl fmt::Display for DailyNodeMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "num_blocks_proposed: {},  num_blocks_failed: {}, failure_rate: {}",
            self.num_blocks_proposed, self.num_blocks_failed, self.failure_rate
        )
    }
}

impl DailyNodeMetrics {
    pub fn new(ts: TimestampNanos, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let daily_total = num_blocks_proposed + num_blocks_failed;
        let failure_rate = if daily_total == 0 {
            Decimal::ZERO
        } else {
            Decimal::from_f64(num_blocks_failed as f64 / daily_total as f64).unwrap()
        };
        DailyNodeMetrics {
            ts: ts.into(),
            num_blocks_proposed,
            num_blocks_failed,
            subnet_assigned,
            failure_rate,
        }
    }
}

/// Represent the node's failure rate on a given day.
///
/// - The failure rate is represented as a `Decimal` in the range [0, 1].
/// - The enum variants are used to give explicit meaning to the nodes' failure rates at every step of the extrapolation
///   algorithm applied in [NodesMultiplierCalculator].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeFailureRate {
    /// The node is assigned to a subnet and has a defined failure rate.
    Defined { subnet_assigned: SubnetId, value: Decimal },
    /// The node is assigned to a subnet and its failure rate has been discounted by the subnet's failure rate.
    DefinedRelative {
        subnet_assigned: SubnetId,
        original_failure_rate: Decimal,
        subnet_failure_rate: Decimal,
        value: Decimal,
    },
    /// The node's failure rate has been extrapolated.
    Extrapolated(Decimal),
    /// The node is not assigned to a subnet.
    Undefined,
}

#[derive(Clone, Debug)]
pub struct DailyNodeFailureRate {
    pub ts: TimestampNanos,
    pub value: NodeFailureRate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DailySubnetFailureRate {
    pub ts: TimestampNanos,
    pub value: Decimal,
}

pub struct DailyMetricsProcessor {
    pub reward_period: RewardPeriod,
    pub daily_metrics_per_node: BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
}

impl DailyMetricsProcessor {
    /// Returns the daily failure rates for all subnets in the reward period.
    ///
    /// - The failure rate of a subnet on a given day is determined as the **75th percentile**
    ///   of the failure rates of all nodes assigned to that subnet on that day.
    /// - If a subnet has no recorded metrics for a particular day within the reward period,
    ///   that day will be excluded from the final result.
    pub fn daily_failure_rates_per_subnet(&self) -> BTreeMap<SubnetId, Vec<DailySubnetFailureRate>> {
        const PERCENTILE: f64 = 0.75;
        let mut failure_rates_map: BTreeMap<(SubnetId, TimestampNanos), Vec<Decimal>> = BTreeMap::new();

        // Collect all failure rates per (subnet_id, ts)
        for daily_metrics in self.daily_metrics_per_node.values().flatten() {
            failure_rates_map
                .entry((daily_metrics.subnet_assigned, *daily_metrics.ts))
                .or_default()
                .push(daily_metrics.failure_rate);
        }

        // Compute the 75th percentile for each (subnet_id, ts)
        let mut daily_failure_rates_per_subnet: BTreeMap<SubnetId, Vec<DailySubnetFailureRate>> = BTreeMap::new();

        for ((subnet_id, ts), mut failure_rates) in failure_rates_map {
            failure_rates.sort();

            let idx_percentile = ((failure_rates.len() as f64) * PERCENTILE).ceil() as usize - 1;
            let failure_rate_percentile = DailySubnetFailureRate {
                ts,
                value: failure_rates[idx_percentile],
            };

            daily_failure_rates_per_subnet.entry(subnet_id).or_default().push(failure_rate_percentile);
        }

        daily_failure_rates_per_subnet
    }

    /// Returns the daily failure rates for a given `node_id` in the reward period.
    ///
    /// - If the node has no metrics for a given day, its failure rate is [NodeFailureRate::Undefined].
    /// - If the node has metrics for a given day, its failure rate is [NodeFailureRate::Defined].
    pub fn daily_failure_rates_in_period(&self, node_id: &NodeId) -> Vec<DailyNodeFailureRate> {
        let days_in_period = &self.reward_period.days_between();

        (0..*days_in_period)
            .map(|day| {
                let ts = TimestampNanosAtDayEnd::from(*self.reward_period.start_ts + day * NANOS_PER_DAY);
                let metrics_for_day = self
                    .daily_metrics_per_node
                    .get(node_id)
                    .and_then(|metrics| metrics.iter().find(|m| m.ts == ts));

                let node_failure_rate = match metrics_for_day {
                    Some(metrics) => NodeFailureRate::Defined {
                        subnet_assigned: metrics.subnet_assigned,
                        value: metrics.failure_rate,
                    },
                    None => NodeFailureRate::Undefined,
                };
                DailyNodeFailureRate {
                    ts: *ts,
                    value: node_failure_rate,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
