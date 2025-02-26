use crate::logs::{LogEntry, Logger, Operation};
use crate::metrics::{NodeDailyFailureRate, NodeFailureRate, SubnetDailyFailureRate};
use crate::tabled_types::generate_table_summary;
use function_name::named;
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::{Ref, RefCell, RefMut};
use std::cmp::max;
use std::collections::BTreeMap;

/// The minimum and maximum failure rates for a node.
/// Nodes with a failure rate below `MIN_FAILURE_RATE` will not be penalized.
/// Nodes with a failure rate above `MAX_FAILURE_RATE` will be penalized with `MAX_REWARDS_REDUCTION`.
const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

/// The minimum and maximum rewards reduction for a node.
const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub struct PerformanceMultipliers {
    /// The computed performance multiplier per node.
    pub performance_multiplier_by_node: BTreeMap<NodeId, Decimal>,
    /// The logger capturing all the computation steps.
    pub logger: Logger,
}

struct ExecutionContext {
    execution_nodes: RefCell<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>>,
    logger: RefCell<Logger>,
}

pub struct PerformanceMultiplierCalculator {
    nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ctx: ExecutionContext,
}

impl PerformanceMultiplierCalculator {
    pub fn new(
        nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
        subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ) -> Self {
        Self {
            nodes_failure_rates,
            subnets_failure_rates,
            ctx: ExecutionContext {
                execution_nodes: RefCell::new(BTreeMap::new()),
                logger: RefCell::new(Logger::default()),
            },
        }
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

    fn logger_mut(&self) -> RefMut<Logger> {
        self.ctx.logger.borrow_mut()
    }

    fn nodes_failure_rates(&self) -> Ref<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>> {
        self.ctx.execution_nodes.borrow()
    }

    fn nodes_failure_rates_mut(&self) -> RefMut<BTreeMap<NodeId, Vec<NodeDailyFailureRate>>> {
        self.ctx.execution_nodes.borrow_mut()
    }

    fn take_ctx(&self) -> (Logger, BTreeMap<NodeId, Vec<NodeDailyFailureRate>>) {
        (
            self.ctx.logger.replace(Logger::default()),
            self.ctx.execution_nodes.replace(BTreeMap::new()),
        )
    }

    fn update_execution_nodes(&self, nodes: &[NodeId]) {
        let execution_nodes = self
            .nodes_failure_rates
            .iter()
            .filter(|(node_id, _)| nodes.contains(node_id))
            .map(|(node_id, failure_rates)| (*node_id, failure_rates.clone()))
            .collect();

        self.ctx.execution_nodes.replace(execution_nodes);
    }

    /// Updates node failure rates to be relative to their subnet’s failure rate.
    ///
    /// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
    /// assigned to.
    /// This is done for removing systematic factors that may affect all nodes in a subnet.
    fn update_relative_failure_rates(&self) {
        for failure_rate in self.nodes_failure_rates_mut().values_mut().flatten() {
            if let NodeFailureRate::Defined { subnet_assigned, value } = failure_rate.value {
                let subnet_failure = self
                    .subnets_failure_rates
                    .get(&subnet_assigned)
                    .and_then(|rates| rates.iter().find(|rate| rate.ts == failure_rate.ts))
                    .map(|rate| rate.value)
                    .expect("Subnet failure rate not found");

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
        self.update_execution_nodes(nodes);

        self.update_relative_failure_rates();

        let extrapolated_failure_rate = self.calculate_extrapolated_failure_rate();

        self.fill_undefined_failure_rates(extrapolated_failure_rate);

        let average_failure_rate_by_node = self.calculate_average_failure_rate_by_node();

        let performance_multiplier_by_node = self.calculate_performance_multiplier_by_node(&average_failure_rate_by_node);

        let (logger, nodes_failure_rates) = self.take_ctx();

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
