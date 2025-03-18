use crate::execution_context::{
    ExecutionContext, Initialized, NodesFRInitialized, PerformanceMultipliersComputed, RelativeFRComputed, UndefinedFRExtrapolated,
};
use crate::intermediate_results::{AllNodesResult, SingleNodeResult};
use crate::metrics::{NodeDailyFailureRate, NodeFailureRate, SubnetDailyFailureRate};
use ic_base_types::{NodeId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::max;
use std::collections::BTreeMap;

/// The minimum and maximum failure rates for a node.
/// Nodes with a failure rate below `MIN_FAILURE_RATE` will not be penalized.
/// Nodes with a failure rate above `MAX_FAILURE_RATE` will be penalized with `MAX_REWARDS_REDUCTION`.
pub const MIN_FAILURE_RATE: Decimal = dec!(0.1);
pub const MAX_FAILURE_RATE: Decimal = dec!(0.6);

/// The minimum and maximum rewards reduction for a node.
pub const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
pub const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

pub struct PerformanceMultiplierCalculator {
    nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
    subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
}

impl PerformanceMultiplierCalculator {
    pub fn new(
        nodes_failure_rates: BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
        subnets_failure_rates: BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ) -> Self {
        Self {
            nodes_failure_rates,
            subnets_failure_rates,
        }
    }
    fn compute_nodes_daily_failure_rate(&self, ctx: ExecutionContext<Initialized>) -> ExecutionContext<NodesFRInitialized> {
        let mut ctx = ctx;
        let nodes = ctx.provider_nodes.iter().map(|node| node.node_id).collect::<Vec<_>>();

        ctx.nodes_failure_rates = self
            .nodes_failure_rates
            .iter()
            .filter(|(node_id, _)| nodes.contains(node_id))
            .map(|(node_id, failure_rates)| (*node_id, failure_rates.clone()))
            .collect();

        ctx.next()
    }

    /// Updates node failure rates to be relative to their subnet’s failure rate.
    ///
    /// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
    /// assigned to.
    /// This is done for removing systematic factors that may affect all nodes in a subnet.
    fn compute_relative_failure_rates(&self, ctx: ExecutionContext<NodesFRInitialized>) -> ExecutionContext<RelativeFRComputed> {
        let mut ctx = ctx;
        for failure_rate in ctx.nodes_failure_rates.values_mut().flatten() {
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
    fn calculate_extrapolated_failure_rate(&self, ctx: &mut ExecutionContext<RelativeFRComputed>) -> Decimal {
        if ctx.nodes_failure_rates.is_empty() {
            return dec!(1);
        }

        let mut nodes_avg_fr = Vec::new();
        for (node_id, failure_rates) in ctx.nodes_failure_rates.iter() {
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
                ctx.tracker.record_node_result(SingleNodeResult::AverageRelativeFR, node_id, &node_avg_fr);
                nodes_avg_fr.push(node_avg_fr);
            }
        }

        let extrapolated_fr = avg(&nodes_avg_fr);
        ctx.tracker.record_all_nodes_result(AllNodesResult::ExtrapolatedFR, &extrapolated_fr);
        extrapolated_fr
    }

    fn fill_undefined_failure_rates(
        &self,
        extrapolated_rate: Decimal,
        ctx: ExecutionContext<RelativeFRComputed>,
    ) -> ExecutionContext<UndefinedFRExtrapolated> {
        let mut ctx = ctx;
        for failure_rate in ctx.nodes_failure_rates.values_mut().flatten() {
            if matches!(failure_rate.value, NodeFailureRate::Undefined) {
                failure_rate.value = NodeFailureRate::Extrapolated(extrapolated_rate);
            }
        }
        ctx.next()
    }

    /// Calculates the average of the failure rates (DefinedRelative and Extrapolated) for each node in the reward period.
    ///
    /// The average failure rate is used to calculate the performance multiplier for each node.
    fn calculate_average_failure_rate_by_node(&self, ctx: &mut ExecutionContext<UndefinedFRExtrapolated>) -> BTreeMap<NodeId, Decimal> {
        ctx.nodes_failure_rates
            .iter()
            .map(|(node_id, failure_rates)| {
                let raw_failure_rates: Vec<Decimal> = failure_rates
                    .iter()
                    .filter_map(|entry| match entry.value {
                        NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                        _ => None,
                    })
                    .collect();

                let average_rate = avg(&raw_failure_rates);
                ctx.tracker
                    .record_node_result(SingleNodeResult::AverageExtrapolatedFR, node_id, &average_rate);
                (*node_id, average_rate)
            })
            .collect()
    }

    /// Calculates the performance multiplier for a node based on its average failure rate.
    fn calculate_performance_multiplier_by_node(
        &self,
        average_failure_rate_by_node: &BTreeMap<NodeId, Decimal>,
        ctx: ExecutionContext<UndefinedFRExtrapolated>,
    ) -> ExecutionContext<PerformanceMultipliersComputed> {
        let mut ctx = ctx;
        ctx.performance_multiplier_by_node = average_failure_rate_by_node
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

                ctx.tracker
                    .record_node_result(SingleNodeResult::RewardsReduction, node_id, &rewards_reduction);
                let performance_multiplier = dec!(1) - rewards_reduction;
                ctx.tracker
                    .record_node_result(SingleNodeResult::PerformanceMultiplier, node_id, &performance_multiplier);

                (*node_id, performance_multiplier)
            })
            .collect();
        ctx.next()
    }

    /// Calculates the performance multipliers for a set of nodes.
    ///
    /// # Arguments
    /// * `nodes` - A vector of node IDs for which the performance multipliers are calculated.
    pub fn calculate(&self, ctx: ExecutionContext<Initialized>) -> ExecutionContext<PerformanceMultipliersComputed> {
        let ctx = self.compute_nodes_daily_failure_rate(ctx);

        let mut ctx = self.compute_relative_failure_rates(ctx);

        let extrapolated_failure_rate = self.calculate_extrapolated_failure_rate(&mut ctx);

        let mut ctx = self.fill_undefined_failure_rates(extrapolated_failure_rate, ctx);

        let average_failure_rate_by_node = self.calculate_average_failure_rate_by_node(&mut ctx);

        self.calculate_performance_multiplier_by_node(&average_failure_rate_by_node, ctx)
    }
}

fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}

#[cfg(test)]
mod tests;
