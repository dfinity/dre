use crate::reward_period::{RewardPeriod, TimestampNanos, TsNanosAtDayStart, NANOS_PER_DAY};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use itertools::Itertools;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct DailyNodeMetrics {
    pub ts: u64,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

impl Default for DailyNodeMetrics {
    fn default() -> Self {
        DailyNodeMetrics {
            ts: 0,
            subnet_assigned: SubnetId::from(PrincipalId::new_anonymous()),
            num_blocks_proposed: 0,
            num_blocks_failed: 0,
            failure_rate: Decimal::ZERO,
        }
    }
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
    pub fn new(ts: u64, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let daily_total = num_blocks_proposed + num_blocks_failed;
        let failure_rate = if daily_total == 0 {
            Decimal::ZERO
        } else {
            Decimal::from_f64(num_blocks_failed as f64 / daily_total as f64).unwrap()
        };
        DailyNodeMetrics {
            ts,
            num_blocks_proposed,
            num_blocks_failed,
            subnet_assigned,
            failure_rate,
        }
    }
}

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

impl TryFrom<NodeFailureRate> for Decimal {
    type Error = String;

    fn try_from(value: NodeFailureRate) -> Result<Self, Self::Error> {
        match value {
            NodeFailureRate::Defined { value, .. } | NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => {
                Ok(value)
            }
            NodeFailureRate::Undefined => Err("Cannot convert undefined failure rate to Decimal".to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DailyNodeFailureRate {
    pub ts: TimestampNanos,
    pub value: NodeFailureRate,
}

#[derive(Clone, Debug)]
pub struct DailySubnetFailureRate {
    pub ts: TimestampNanos,
    pub value: Decimal,
}

pub struct MetricsProcessor {
    pub daily_metrics_per_node: BTreeMap<NodeId, Vec<DailyNodeMetrics>>,
    pub reward_period: RewardPeriod,
}

impl MetricsProcessor {
    pub fn daily_failure_rates_in_period(&self, node_id: &NodeId) -> Vec<DailyNodeFailureRate> {
        let days_in_period = &self.reward_period.days_between();

        (0..*days_in_period)
            .map(|day| {
                let ts = *self.reward_period.start_ts + day * NANOS_PER_DAY;
                let metrics_for_day = self
                    .daily_metrics_per_node
                    .get(node_id)
                    .and_then(|metrics| metrics.iter().find(|m| *TsNanosAtDayStart::from(m.ts) == ts));

                let node_failure_rate = match metrics_for_day {
                    Some(metrics) => NodeFailureRate::Defined {
                        subnet_assigned: metrics.subnet_assigned,
                        value: metrics.failure_rate,
                    },
                    None => NodeFailureRate::Undefined,
                };
                DailyNodeFailureRate {
                    ts,
                    value: node_failure_rate,
                }
            })
            .collect()
    }

    pub fn daily_failure_rates_per_subnet(&self) -> BTreeMap<SubnetId, Vec<DailySubnetFailureRate>> {
        const PERCENTILE: f64 = 0.75;

        self.daily_metrics_per_node
            .values()
            .flatten()
            .map(|daily_metrics| (daily_metrics.subnet_assigned, daily_metrics.ts, daily_metrics.failure_rate))
            .chunk_by(|(subnet_assigned, ts, _)| (*subnet_assigned, *ts))
            .into_iter()
            .map(|((subnet_id, ts), group)| {
                let subnet_failure_rates: Vec<Decimal> = group.map(|(_, _, failure_rate)| failure_rate).sorted().collect();
                let idx_percentile = ((subnet_failure_rates.len() as f64) * PERCENTILE).ceil() as usize - 1;

                let failure_rate_percentile = DailySubnetFailureRate {
                    ts,
                    value: subnet_failure_rates[idx_percentile],
                };

                (subnet_id, failure_rate_percentile)
            })
            .into_group_map()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests;
