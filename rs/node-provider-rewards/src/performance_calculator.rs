use crate::logs::{LogEntry, Logger, Operation};
use crate::metrics::{NodeDailyFailureRate, NodeDailyMetrics, NodeFailureRate, SubnetDailyFailureRate};
use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use crate::tabled_types::generate_table_summary;
use function_name::named;
use ic_base_types::{NodeId, SubnetId};
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::{Ref, RefCell, RefMut};
use std::cmp::max;
use std::collections::BTreeMap;

/// The percentile used to calculate the failure rate for a subnet.
const SUBNET_FAILURE_RATE_PERCENTILE: f64 = 0.75;

/// The minimum and maximum failure rates for a node.
/// Nodes with a failure rate below `MIN_FAILURE_RATE` will not be penalized.
/// Nodes with a failure rate above `MAX_FAILURE_RATE` will be penalized with `MAX_REWARDS_REDUCTION`.
const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

/// The minimum and maximum rewards reduction for a node.
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
    fn calculate_subnets_failure_rates(&self) -> BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>> {
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
            let percentile_rate = SubnetDailyFailureRate { ts, value: rates[index] };

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
                let metrics_for_day: Option<Vec<&NodeDailyMetrics>> = self
                    .metrics_by_node
                    .get(node_id)
                    .map(|metrics_in_period| metrics_in_period.iter().filter(|m| m.ts == ts).collect_vec());

                let node_failure_rate = match metrics_for_day {
                    // Node is assigned in reward period but has no metrics for the day.
                    Some(metrics_for_day) if metrics_for_day.is_empty() => NodeFailureRate::Undefined,
                    // Node is assigned to only one subnet.
                    Some(metrics_for_day) if metrics_for_day.len() == 1 => {
                        let first = metrics_for_day.first().expect("No metrics");

                        NodeFailureRate::Defined {
                            subnet_assigned: first.subnet_assigned,
                            value: first.failure_rate,
                        }
                    }
                    // In this case the node is reassigned to different subnets on the same day.
                    // The algorithm considers for this case the subnet where the node has proposed more blocks.
                    Some(metrics_for_day) => {
                        let mut subnet_block_counts: BTreeMap<SubnetId, u64> = BTreeMap::new();

                        for metrics in metrics_for_day.iter() {
                            *subnet_block_counts.entry(metrics.subnet_assigned).or_insert(0) += metrics.num_blocks_proposed;
                        }

                        let (subnet_assigned, _) = subnet_block_counts.into_iter().max_by_key(|&(_, count)| count).expect("No subnet found");

                        let failure_rate = metrics_for_day
                            .iter()
                            .find(|m| m.subnet_assigned == subnet_assigned)
                            .expect("No metrics for the selected subnet")
                            .failure_rate;

                        NodeFailureRate::Defined {
                            subnet_assigned,
                            value: failure_rate,
                        }
                    }
                    // Node has not being assigned to any subnet in reward period.
                    None => NodeFailureRate::Undefined,
                };

                NodeDailyFailureRate {
                    ts: *ts,
                    value: node_failure_rate,
                }
            })
            .collect()
    }
}

pub struct PerformanceMultipliers {
    /// The computed performance multiplier per node.
    pub _performance_multiplier_by_node: BTreeMap<NodeId, Decimal>,
    /// The logger capturing all the computation steps.
    pub _logger: Logger,
}

pub struct PerformanceMultiplierCalculator {
    mgr: FailureRatesManager,
    subnets_failure_rates: Option<BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>>,
    nodes_failure_rates: RefCell<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>>,
    logger: RefCell<Logger>,
}

impl PerformanceMultiplierCalculator {
    pub fn new(failure_rates_manager: FailureRatesManager) -> Self {
        Self {
            mgr: failure_rates_manager,
            subnets_failure_rates: None,
            nodes_failure_rates: RefCell::new(BTreeMap::default()),
            logger: RefCell::new(Logger::default()),
        }
    }

    pub fn with_subnets_failure_rates_discount(self) -> Self {
        let subnets_failure_rates = self.mgr.calculate_subnets_failure_rates();
        Self {
            subnets_failure_rates: Some(subnets_failure_rates),
            ..self
        }
    }

    fn update_nodes_failure_rates(&self, nodes: &[NodeId]) {
        let nodes_failure_rates = nodes
            .iter()
            .map(|node_id| (*node_id, self.mgr.node_failure_rates_in_period(node_id)))
            .collect();

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
    ///
    /// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
    /// assigned to.
    /// This is done for removing systematic factors that may affect all nodes in a subnet.
    fn update_relative_failure_rates(&self) {
        for failure_rate in self.nodes_failure_rates_mut().values_mut().flatten() {
            if let NodeFailureRate::Defined { subnet_assigned, value } = failure_rate.value {
                let subnet_failure = match &self.subnets_failure_rates {
                    Some(subnets_failure_rates) => self
                        .lookup_subnet_failure_rate(subnet_assigned, failure_rate.ts, subnets_failure_rates)
                        .expect("Subnet failure rate not found"),
                    None => Decimal::ZERO,
                };

                let relative_failure = max(Decimal::ZERO, value - subnet_failure);

                failure_rate.value = NodeFailureRate::DefinedRelative {
                    subnet_assigned,
                    subnet_failure_rate: subnet_failure,
                    original_failure_rate: value,
                    value: relative_failure,
                };
            }
        }
    }

    /// Calculates the extrapolated failure rate used as replacement for nodes with `Undefined` failure rates.
    ///
    /// For each node is computed the average of the relative failure rates recorded in the reward period.
    /// The extrapolated failure rate is the average of these averages.
    /// This is done to put higher weight on nodes with less recorded failure rates (assigned for fewer days).
    #[named]
    fn calculate_extrapolated_failure_rate(&self) -> Decimal {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        if self.nodes_failure_rates().is_empty() {
            return self.run_and_log("No nodes assigned", Operation::Set(dec!(1)));
        }

        let nodes_avg_failure_rates: Vec<Decimal> = self
            .nodes_failure_rates()
            .iter()
            .map(|(node_id, failure_rates)| {
                let raw_failure_rates: Vec<Decimal> = failure_rates
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } => Some(value),
                        _ => None,
                    })
                    .collect();

                self.run_and_log(&node_id.to_string(), Operation::Avg(raw_failure_rates))
            })
            .collect();

        self.run_and_log("Extrapolated Failure Rate", Operation::Avg(nodes_avg_failure_rates))
    }

    fn fill_undefined_failure_rates(&self, extrapolated_rate: Decimal) {
        for failure_rate in self.nodes_failure_rates_mut().values_mut().flatten() {
            if matches!(failure_rate.value, NodeFailureRate::Undefined) {
                failure_rate.value = NodeFailureRate::Extrapolated(extrapolated_rate);
            }
        }
    }

    /// Calculates the average of the failure rates (DefinedRelative and Extrapolated) for each node in the reward period.
    ///
    /// The average failure rate is used to calculate the performance multiplier for each node.
    #[named]
    fn calculate_average_failure_rate_by_node(&self) -> BTreeMap<NodeId, Decimal> {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        self.nodes_failure_rates()
            .iter()
            .map(|(node_id, failure_rates)| {
                let raw_failure_rates: Vec<Decimal> = failure_rates
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                        _ => None,
                    })
                    .collect();

                let average_rate = self.run_and_log(&node_id.to_string(), Operation::Avg(raw_failure_rates));
                (*node_id, average_rate)
            })
            .collect()
    }

    /// Calculates the performance multiplier for a node based on its average failure rate.
    #[named]
    fn calculate_performance_multiplier_by_node(&self, average_failure_rate_by_node: &BTreeMap<NodeId, Decimal>) -> BTreeMap<NodeId, Decimal> {
        self.logger_mut().log(LogEntry::NodesMultiplierStep(function_name!()));

        average_failure_rate_by_node
            .iter()
            .map(|(node_id, average_failure_rate)| {
                let rewards_reduction;

                if average_failure_rate < &MIN_FAILURE_RATE {
                    rewards_reduction = MIN_REWARDS_REDUCTION;
                } else if average_failure_rate > &MAX_FAILURE_RATE {
                    rewards_reduction = MAX_REWARDS_REDUCTION;
                } else {
                    // Linear interpolation between MIN_REWARDS_REDUCTION and MAX_REWARDS_REDUCTION
                    rewards_reduction = ((*average_failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)) * MAX_REWARDS_REDUCTION;
                };

                let performance_multiplier = dec!(1) - rewards_reduction;

                self.logger_mut().log(LogEntry::PerformanceMultiplier {
                    node_id: *node_id,
                    failure_rate_in_period: *average_failure_rate,
                    rewards_reduction,
                    multiplier: performance_multiplier,
                });

                (*node_id, performance_multiplier)
            })
            .collect()
    }

    /// Calculates the performance multipliers for a set of nodes.
    ///
    /// # Arguments
    /// * `nodes` - A vector of node IDs for which the performance multipliers are calculated.
    pub fn calculate_performance_multipliers(&self, nodes: &[NodeId]) -> PerformanceMultipliers {
        self.update_nodes_failure_rates(nodes);

        self.update_relative_failure_rates();

        let extrapolated_failure_rate = self.calculate_extrapolated_failure_rate();

        self.fill_undefined_failure_rates(extrapolated_failure_rate);

        let average_failure_rate_by_node = self.calculate_average_failure_rate_by_node();

        let performance_multiplier_by_node = self.calculate_performance_multiplier_by_node(&average_failure_rate_by_node);

        let (logger, nodes_failure_rates) = (self.logger.take(), self.nodes_failure_rates.take());

        for (node_id, failure_entries) in nodes_failure_rates.into_iter() {
            let failure_entries_table = generate_table_summary(failure_entries);
            self.logger_mut().log(LogEntry::Summary(node_id, Box::new(failure_entries_table)));
        }

        PerformanceMultipliers {
            _logger: logger,
            _performance_multiplier_by_node: performance_multiplier_by_node,
        }
    }
}

#[cfg(test)]
mod tests;
