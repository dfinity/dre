use super::*;
use crate::execution_context::results_tracker::{NodeResult, SingleResult};
use crate::metrics::NodeFailureRate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::max;

/// The minimum and maximum failure rates for a node.
/// Nodes with a failure rate below `MIN_FAILURE_RATE` will not be penalized.
/// Nodes with a failure rate above `MAX_FAILURE_RATE` will be penalized with `MAX_REWARDS_REDUCTION`.
pub const MIN_FAILURE_RATE: Decimal = dec!(0.1);
pub const MAX_FAILURE_RATE: Decimal = dec!(0.6);

/// The minimum and maximum rewards reduction for a node.
pub const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
pub const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub(super) struct PerformanceCalculatorContext<'a, T: ExecutionState> {
    pub(super) subnets_fr: &'a BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    pub(super) execution_nodes_fr: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    pub(super) results_tracker: ResultsTracker,
    pub(super) _marker: PhantomData<T>,
}

impl<'a, T: ExecutionState> PerformanceCalculatorContext<'a, T> {
    pub fn transition<S: ExecutionState>(self) -> PerformanceCalculatorContext<'a, S> {
        PerformanceCalculatorContext {
            subnets_fr: self.subnets_fr,
            execution_nodes_fr: self.execution_nodes_fr,
            results_tracker: self.results_tracker,
            _marker: PhantomData,
        }
    }
}

impl<'a> PerformanceCalculatorContext<'a, StartPerformanceCalculator> {
    pub fn next(self) -> PerformanceCalculatorContext<'a, ComputeRelativeFR> {
        PerformanceCalculatorContext::transition(self)
    }
}

impl<'a> PerformanceCalculatorContext<'a, ComputeRelativeFR> {
    /// Updates node failure rates to be relative to their subnet’s failure rate.
    ///
    /// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
    /// assigned to.
    /// This is done for removing systematic factors that may affect all nodes in a subnet.
    pub fn next(mut self) -> PerformanceCalculatorContext<'a, ComputeExtrapolatedFR> {
        for failure_rate in self.execution_nodes_fr.values_mut().flatten() {
            if let NodeFailureRate::Defined { subnet_assigned, value } = failure_rate.value {
                let subnet_failure = self
                    .subnets_fr
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

        PerformanceCalculatorContext::transition(self)
    }
}

impl<'a> PerformanceCalculatorContext<'a, ComputeExtrapolatedFR> {
    /// Calculates the extrapolated failure rate used as replacement for nodes with `Undefined` failure rates.
    ///
    /// For each node is computed the average of the relative failure rates recorded in the reward period.
    /// The extrapolated failure rate is the average of these averages.
    /// This is done to put higher weight on nodes with less recorded failure rates (assigned for fewer days).
    pub fn next(mut self) -> PerformanceCalculatorContext<'a, FillUndefinedFR> {
        if self.execution_nodes_fr.is_empty() {
            self.results_tracker.record_single_result(SingleResult::ExtrapolatedFR, &dec!(1));

            return PerformanceCalculatorContext::transition(self);
        }

        let mut nodes_avg_fr = Vec::new();
        for (node_id, failure_rates) in self.execution_nodes_fr.iter() {
            let failure_rates: Vec<Decimal> = failure_rates
                .iter()
                .filter_map(|entry| match entry.value {
                    NodeFailureRate::DefinedRelative { value, .. } => Some(value),
                    _ => None,
                })
                .collect();

            // Do not consider nodes completely unassigned
            if !failure_rates.is_empty() {
                let node_avg_fr = avg(&failure_rates);
                self.results_tracker
                    .record_node_result(NodeResult::AverageRelativeFR, node_id, &node_avg_fr);
                nodes_avg_fr.push(node_avg_fr);
            }
        }

        let extrapolated_fr = avg(&nodes_avg_fr);
        self.results_tracker.record_single_result(SingleResult::ExtrapolatedFR, &extrapolated_fr);
        PerformanceCalculatorContext::transition(self)
    }
}

impl<'a> PerformanceCalculatorContext<'a, FillUndefinedFR> {
    /// Fills the `Undefined` failure rates with the extrapolated failure rate.
    pub fn next(mut self) -> PerformanceCalculatorContext<'a, ComputeAverageExtrapolatedFR> {
        let extrapolated_fr = self.results_tracker.get_single_result(SingleResult::ExtrapolatedFR);

        for failure_rate in self.execution_nodes_fr.values_mut().flatten() {
            if matches!(failure_rate.value, NodeFailureRate::Undefined) {
                failure_rate.value = NodeFailureRate::Extrapolated(*extrapolated_fr);
            }
        }
        PerformanceCalculatorContext::transition(self)
    }
}

impl<'a> PerformanceCalculatorContext<'a, ComputeAverageExtrapolatedFR> {
    /// Calculates the average of the failure rates (DefinedRelative and Extrapolated) for each node in the reward period.
    ///
    /// The average failure rate is used to calculate the performance multiplier for each node.
    pub fn next(mut self) -> PerformanceCalculatorContext<'a, ComputePerformanceMultipliers> {
        for (node_id, failure_rates) in self.execution_nodes_fr.iter() {
            let raw_failure_rates: Vec<Decimal> = failure_rates
                .iter()
                .filter_map(|entry| match entry.value {
                    NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                    _ => None,
                })
                .collect();

            let average_rate = avg(&raw_failure_rates);
            self.results_tracker
                .record_node_result(NodeResult::AverageExtrapolatedFR, node_id, &average_rate);
        }

        PerformanceCalculatorContext::transition(self)
    }
}

impl<'a> PerformanceCalculatorContext<'a, ComputePerformanceMultipliers> {
    /// Calculates the performance multiplier for a node based on its average failure rate.
    pub fn next(mut self) -> PerformanceCalculatorContext<'a, PerformanceMultipliersComputed> {
        let average_extrapolated_fr = self.results_tracker.get_nodes_result(NodeResult::AverageExtrapolatedFR).clone();

        for (node_id, average_failure_rate) in average_extrapolated_fr {
            let rewards_reduction;

            if average_failure_rate < MIN_FAILURE_RATE {
                rewards_reduction = MIN_REWARDS_REDUCTION;
            } else if average_failure_rate > MAX_FAILURE_RATE {
                rewards_reduction = MAX_REWARDS_REDUCTION;
            } else {
                // Linear interpolation between MIN_REWARDS_REDUCTION and MAX_REWARDS_REDUCTION
                rewards_reduction = ((average_failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)) * MAX_REWARDS_REDUCTION;
            };

            self.results_tracker
                .record_node_result(NodeResult::RewardsReduction, &node_id, &rewards_reduction);
            let performance_multiplier = dec!(1) - rewards_reduction;
            self.results_tracker
                .record_node_result(NodeResult::PerformanceMultiplier, &node_id, &performance_multiplier);
        }

        PerformanceCalculatorContext::transition(self)
    }
}

pub(super) struct StartPerformanceCalculator;
impl ExecutionState for StartPerformanceCalculator {}
pub(super) struct ComputeRelativeFR;
impl ExecutionState for ComputeRelativeFR {}
pub(super) struct ComputeExtrapolatedFR;
impl ExecutionState for ComputeExtrapolatedFR {}
pub(super) struct FillUndefinedFR {}
impl ExecutionState for FillUndefinedFR {}
pub(super) struct ComputeAverageExtrapolatedFR;
impl ExecutionState for ComputeAverageExtrapolatedFR {}
pub(super) struct ComputePerformanceMultipliers;
impl ExecutionState for ComputePerformanceMultipliers {}

#[cfg(test)]
mod tests;
