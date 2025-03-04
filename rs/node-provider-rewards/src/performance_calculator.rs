use crate::logs::{LogEntry, Logger, Operation};
use crate::metrics::{NodeDailyFailureRate, NodeFailureRate, SubnetDailyFailureRate};
use crate::tabled_types::generate_table_summary;
use function_name::named;
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::max;
use std::collections::BTreeMap;
use std::marker::PhantomData;

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
    pub _performance_multiplier_by_node: BTreeMap<NodeId, Decimal>,
    /// The logger capturing all the computation steps.
    pub _logger: Logger,
}

struct ExecutionContext<T: ExecutionState> {
    execution_nodes: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    logger: Logger,
    _marker: PhantomData<T>
}

impl<T: ExecutionState> ExecutionContext<T> {
    fn transition<S: ExecutionState>(self) -> ExecutionContext<S> {
        ExecutionContext {
            logger: self.logger,
            execution_nodes: self.execution_nodes,
            _marker: PhantomData,
        }
    }
}

trait ExecutionState {}
struct Initialized;
struct RelativeFRComputed;
struct UndefinedFRExtrapolated;

impl ExecutionState for Initialized {}
impl ExecutionState for RelativeFRComputed {}
impl ExecutionState for UndefinedFRExtrapolated {}

impl ExecutionContext<Initialized> {
    // Initialized -> RelativeFRComputed
    pub fn next(self) -> ExecutionContext<RelativeFRComputed> {
        ExecutionContext::transition(self)
    }
}

impl ExecutionContext<RelativeFRComputed> {
    // RelativeFRComputed -> UndefinedFRExtrapolated
    pub fn next(self) -> ExecutionContext<UndefinedFRExtrapolated> {
        ExecutionContext::transition(self)
    }
}


pub struct PerformanceMultiplierCalculator {
    nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>
}

impl PerformanceMultiplierCalculator {
    pub fn new(
        nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
        subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ) -> Self {
        Self {
            nodes_failure_rates,
            subnets_failure_rates
        }
    }
    fn execution_context(&self, nodes: &[NodeId]) -> ExecutionContext<Initialized> {
        let execution_nodes = self
            .nodes_failure_rates
            .iter()
            .filter(|(node_id, _)| nodes.contains(node_id))
            .map(|(node_id, failure_rates)| (*node_id, failure_rates.clone()))
            .collect();
        
        ExecutionContext {
            execution_nodes,
            logger: Logger::default(),
            _marker: PhantomData
        }
    }

    /// Updates node failure rates to be relative to their subnet’s failure rate.
    ///
    /// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
    /// assigned to.
    /// This is done for removing systematic factors that may affect all nodes in a subnet.
    fn compute_relative_failure_rates(&self, ctx: ExecutionContext<Initialized>) -> ExecutionContext<RelativeFRComputed> {
        let mut ctx = ctx;
        for failure_rate in ctx.execution_nodes.values_mut().flatten() {
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
        ctx.next()
    }

    /// Calculates the extrapolated failure rate used as replacement for nodes with `Undefined` failure rates.
    ///
    /// For each node is computed the average of the relative failure rates recorded in the reward period.
    /// The extrapolated failure rate is the average of these averages.
    /// This is done to put higher weight on nodes with less recorded failure rates (assigned for fewer days).
    #[named]
    fn calculate_extrapolated_failure_rate(&self, ctx: &mut ExecutionContext<RelativeFRComputed>) -> Decimal {
        ctx.logger.log(LogEntry::NodesMultiplierStep(function_name!()));

        if ctx.execution_nodes.is_empty() {
            return ctx.logger.run_and_log("No nodes assigned", Operation::Set(dec!(1)));
        }
        
        let mut nodes_avg_failure_rates = Vec::new();
        for (node_id, failure_rates) in ctx.execution_nodes.iter() {
            let failure_rates: Vec<Decimal> = failure_rates
                .iter()
                .filter_map(|entry| match entry.value {
                    NodeFailureRate::DefinedRelative { value, .. } => Some(value),
                    _ => None,
                })
                .collect();

            nodes_avg_failure_rates.push(ctx.logger.run_and_log(&node_id.to_string(), Operation::Avg(failure_rates)));
        }
        
        ctx.logger.run_and_log("Extrapolated Failure Rate", Operation::Avg(nodes_avg_failure_rates))
    }

    fn fill_undefined_failure_rates(&self, extrapolated_rate: Decimal, ctx: ExecutionContext<RelativeFRComputed>) -> ExecutionContext<UndefinedFRExtrapolated> {
        let mut ctx = ctx;
        for failure_rate in ctx.execution_nodes.values_mut().flatten() {
            if matches!(failure_rate.value, NodeFailureRate::Undefined) {
                failure_rate.value = NodeFailureRate::Extrapolated(extrapolated_rate);
            }
        }
        ctx.next()
    }

    /// Calculates the average of the failure rates (DefinedRelative and Extrapolated) for each node in the reward period.
    ///
    /// The average failure rate is used to calculate the performance multiplier for each node.
    #[named]
    fn calculate_average_failure_rate_by_node(&self, ctx: &mut ExecutionContext<UndefinedFRExtrapolated>) -> BTreeMap<NodeId, Decimal> {
        ctx.logger.log(LogEntry::NodesMultiplierStep(function_name!()));

        ctx.execution_nodes
            .iter()
            .map(|(node_id, failure_rates)| {
                let raw_failure_rates: Vec<Decimal> = failure_rates
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                        _ => None,
                    })
                    .collect();

                let average_rate = ctx.logger.run_and_log(&node_id.to_string(), Operation::Avg(raw_failure_rates));
                (*node_id, average_rate)
            })
            .collect()
    }

    /// Calculates the performance multiplier for a node based on its average failure rate.
    #[named]
    fn calculate_performance_multiplier_by_node(&self, average_failure_rate_by_node: &BTreeMap<NodeId, Decimal>, ctx: &mut ExecutionContext<UndefinedFRExtrapolated>) -> BTreeMap<NodeId, Decimal> {
        ctx.logger.log(LogEntry::NodesMultiplierStep(function_name!()));

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

                ctx.logger.log(LogEntry::PerformanceMultiplier {
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
        let ctx = self.execution_context(nodes);
        
        let mut ctx = self.compute_relative_failure_rates(ctx);
        
        let extrapolated_failure_rate = self.calculate_extrapolated_failure_rate(&mut ctx);

        let mut ctx = self.fill_undefined_failure_rates(extrapolated_failure_rate, ctx);

        let average_failure_rate_by_node = self.calculate_average_failure_rate_by_node(&mut ctx);

        let performance_multiplier_by_node = self.calculate_performance_multiplier_by_node(&average_failure_rate_by_node, &mut ctx);
        
        for (node_id, failure_entries) in ctx.execution_nodes.into_iter() {
            let failure_entries_table = generate_table_summary(failure_entries);
            ctx.logger.log(LogEntry::Summary(node_id, Box::new(failure_entries_table)));
        }

        PerformanceMultipliers {
            _logger: ctx.logger,
            _performance_multiplier_by_node: performance_multiplier_by_node,
        }
    }
}

#[cfg(test)]
mod tests;
