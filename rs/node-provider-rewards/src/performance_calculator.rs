use crate::logs::{LogEntry, Logger, Operation};
use crate::metrics::{NodeDailyFailureRate, NodeDailyMetrics, NodeFailureRate, SubnetDailyFailureRate};
use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use crate::tabled_types::generate_table_summary;
use function_name::named;
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;

const SUBNET_FAILURE_RATE_PERCENTILE: f64 = 0.75;

const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub struct FailureRatesManager {
    reward_period: RewardPeriod,
    metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>>,
}

impl FailureRatesManager {
    pub fn new(reward_period: RewardPeriod, metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>>) -> Self {
        Self {
            reward_period,
            metrics_by_node,
        }
    }

    /// Computes the failure rates for each subnet on a given day.
    ///
    /// The failure rate for a subnet on a given day is defined as the **75th percentile**
    /// of the failure rates of all nodes assigned to that subnet. Days with no recorded
    /// metrics for a subnet are omitted.
    pub fn calculate_subnets_failure_rates(&self) -> BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> {
        let mut rates_map: BTreeMap<(SubnetId, TimestampNanos), Vec<Decimal>> = BTreeMap::new();

        // Aggregate failure rates by (subnet, timestamp)
        for metrics in self.metrics_by_node.values().flatten() {
            rates_map
                .entry((metrics.subnet_assigned, *metrics.ts))
                .or_default()
                .push(metrics.failure_rate);
        }

        let mut subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> = BTreeMap::new();

        for ((subnet, ts), mut rates) in rates_map {
            rates.sort();

            let index = ((rates.len() as f64) * SUBNET_FAILURE_RATE_PERCENTILE).ceil() as usize - 1;
            let percentile_rate = SubnetDailyFailureRate { ts: ts, value: rates[index] };

            subnets_failure_rates.entry(subnet).or_default().push(percentile_rate);
        }

        subnets_failure_rates
    }

    /// Calculates daily failure rates for a given node over the reward period.
    ///
    /// If a node has no metrics recorded for a day, its failure rate is marked as [NodeFailureRate::Undefined].
    /// Otherwise, it is recorded as [NodeFailureRate::Defined].
    fn node_failure_rates_in_period(&self, node_id: &NodeId) -> Vec<NodeDailyFailureRate> {
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
    pub fn calculate_nodes_failure_rates(&self, nodes: &Vec<NodeId>) -> BTreeMap<NodeId, Vec<NodeDailyFailureRate>> {
        nodes
            .iter()
            .map(|node_id| (*node_id, self.node_failure_rates_in_period(node_id)))
            .collect()
    }
}

/// The result of the rewards multiplier calculation.
pub struct PerformanceMultiplier {
    /// The computed rewards multipliers per node.
    pub performance_multiplier_by_node: BTreeMap<NodeId, Decimal>,
    /// The logger capturing all the computation steps.
    pub logger: Logger,
}

pub struct PerformanceMultiplierCalculator {
    failure_rates_manager: FailureRatesManager,
    subnets_failure_rates: Option<BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>>,
    nodes_failure_rates: RefCell<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>>,
    logger: RefCell<Logger>,
}

impl PerformanceMultiplierCalculator {
    pub fn new(failure_rates_manager: FailureRatesManager) -> Self {
        Self {
            failure_rates_manager,
            subnets_failure_rates: None,
            nodes_failure_rates: RefCell::new(BTreeMap::default()),
            logger: RefCell::new(Logger::default()),
        }
    }

    pub fn with_subnets_failure_rates_discount(self) -> Self {
        let subnets_failure_rates = self.failure_rates_manager.calculate_subnets_failure_rates();
        Self {
            subnets_failure_rates: Some(subnets_failure_rates),
            ..self
        }
    }

    fn update_nodes_failure_rates(&self, nodes: &Vec<NodeId>) {
        let nodes_failure_rates = self.failure_rates_manager.calculate_nodes_failure_rates(nodes);
        self.nodes_failure_rates.replace(nodes_failure_rates);
    }

    fn logger_mut(&self) -> RefMut<Logger> {
        self.logger.borrow_mut()
    }

    fn nodes_failure_rates_mut(&self) -> RefMut<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>> {
        self.nodes_failure_rates.borrow_mut()
    }

    fn nodes_failure_rates(&self) -> Ref<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>> {
        self.nodes_failure_rates.borrow()
    }

    fn run_and_log(&self, description: &str, operation: Operation) -> Decimal {
        let result = operation.execute();
        self.logger_mut().log(LogEntry::Execute {
            reason: format!("\t{}", description),
            operation,
            result,
        });
        result
    }

    fn lookup_subnet_failure_rate(
        &self,
        subnet: SubnetId,
        timestamp: TimestampNanos,
        subnets_failure_rates: &BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ) -> Option<Decimal> {
        subnets_failure_rates
            .get(&subnet)
            .and_then(|rates| rates.iter().find(|rate| rate.ts == timestamp))
            .map(|rate| rate.value)
    }

    /// Updates node failure rates to be relative to their subnet’s failure rate.
    fn update_relative_failure_rates(&self) {
        for failure_entry in self.nodes_failure_rates_mut().values_mut().flatten() {
            if let NodeFailureRate::Defined { subnet_assigned, value } = failure_entry.value {
                let subnet_failure = match &self.subnets_failure_rates {
                    Some(subnets_failure_rates) => self
                        .lookup_subnet_failure_rate(subnet_assigned, failure_entry.ts, subnets_failure_rates)
                        .expect("Subnet failure rate not found"),
                    None => Decimal::ZERO,
                };

                let relative_failure = if subnet_failure < value { value - subnet_failure } else { Decimal::ZERO };

                failure_entry.value = NodeFailureRate::DefinedRelative {
                    subnet_assigned,
                    subnet_failure_rate: subnet_failure,
                    original_failure_rate: value,
                    value: relative_failure,
                };
            }
        }
    }

    #[named]
    fn calculate_extrapolated_failure_rate(&self) -> Decimal {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        if self.nodes_failure_rates().is_empty() {
            return self.run_and_log("No nodes assigned", Operation::Set(dec!(1)));
        }

        let node_relative_averages: Vec<Decimal> = self
            .nodes_failure_rates()
            .iter()
            .map(|(node_id, failure_entries)| {
                let relative_rates: Vec<Decimal> = failure_entries
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } => Some(value),
                        _ => None,
                    })
                    .collect();

                self.run_and_log(&node_id.to_string(), Operation::Avg(relative_rates))
            })
            .collect();

        self.run_and_log("Extrapolated Failure Rate", Operation::Avg(node_relative_averages))
    }

    fn fill_undefined_failure_rates(&self, extrapolated_rate: Decimal) {
        for failure_entry in self.nodes_failure_rates_mut().values_mut().flatten() {
            if matches!(failure_entry.value, NodeFailureRate::Undefined) {
                failure_entry.value = NodeFailureRate::Extrapolated(extrapolated_rate);
            }
        }
    }

    #[named]
    fn calculate_average_failure_rate_by_node(&self) -> BTreeMap<NodeId, Decimal> {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        self.nodes_failure_rates()
            .iter()
            .map(|(node_id, failure_entries)| {
                let rates: Vec<Decimal> = failure_entries
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                        _ => None,
                    })
                    .collect();

                let average_rate = self.run_and_log(&node_id.to_string(), Operation::Avg(rates));
                (*node_id, average_rate)
            })
            .collect()
    }

    #[named]
    fn calculate_performance_multiplier_by_node(&self, average_failure_rates: &BTreeMap<NodeId, Decimal>) -> BTreeMap<NodeId, Decimal> {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        average_failure_rates
            .iter()
            .map(|(node_id, avg_rate)| {
                let rewards_reduction = if avg_rate < &MIN_FAILURE_RATE {
                    MIN_REWARDS_REDUCTION
                } else if avg_rate > &MAX_FAILURE_RATE {
                    MAX_REWARDS_REDUCTION
                } else {
                    ((*avg_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)) * MAX_REWARDS_REDUCTION
                };
                let multiplier = dec!(1) - rewards_reduction;

                self.logger_mut().log(LogEntry::RewardsMultiplier {
                    node_id: *node_id,
                    failure_rate_in_period: *avg_rate,
                    rewards_reduction,
                    rewards_multiplier: multiplier,
                });
                (*node_id, multiplier)
            })
            .collect()
    }

    /// Computes the rewards multipliers for nodes.
    ///
    /// # Arguments
    /// * `node_failure_data` - A mapping from node IDs to their respective vectors of daily failure rates.
    pub fn calculate_performance_multiplier(&self, nodes: &Vec<NodeId>) -> PerformanceMultiplier {
        self.update_nodes_failure_rates(nodes);

        self.update_relative_failure_rates();

        let extrapolated_failure_rate = self.calculate_extrapolated_failure_rate();

        self.fill_undefined_failure_rates(extrapolated_failure_rate);

        let average_failure_rate_by_node = self.calculate_average_failure_rate_by_node();

        let performance_multiplier_by_node = self.calculate_performance_multiplier_by_node(&average_failure_rate_by_node);

        let (logger, nodes_failure_rates) = (self.logger.take(), self.nodes_failure_rates.take());

        for (node_id, failure_entries) in nodes_failure_rates.into_iter() {
            self.logger_mut().log(LogEntry::Summary(node_id, generate_table_summary(failure_entries)));
        }

        PerformanceMultiplier {
            logger,
            performance_multiplier_by_node,
        }
    }
}

#[cfg(test)]
mod tests;
