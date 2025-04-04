use crate::rewards_calculator_results::{NodeCategory, RewardsCalculatorResults};
use crate::types::{DayEndNanos, NodeMetricsDailyProcessed, NodeStatus, RewardPeriod, TimestampNanos, NANOS_PER_DAY};
use crate::types::{NodeFailureRate, NodeMetricsDaily, RewardableNode, SubnetFailureRateDaily};

use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::Ref;
use std::cmp::max;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

/// The minimum and maximum failure rates for a node.
/// Nodes with a failure rate below `MIN_FAILURE_RATE` will not be penalized.
/// Nodes with a failure rate above `MAX_FAILURE_RATE` will be penalized with `MAX_REWARDS_REDUCTION`.
const MIN_FAILURE_RATE: Decimal = dec!(0.1);
const MAX_FAILURE_RATE: Decimal = dec!(0.6);

/// The minimum and maximum rewards reduction for a node.
const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

const FULL_REWARDS_MACHINES_LIMIT: u32 = 4;

fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}

pub(super) struct NodeProviderRewardsCalculator<'a, T: ExecutionState> {
    pub(super) provider_id: PrincipalId,
    pub(super) rewardable_nodes: Vec<RewardableNode>,
    pub(super) calculator_results: RewardsCalculatorResults,
    pub(super) daily_subnets_fr: BTreeMap<SubnetId, Vec<SubnetFailureRateDaily>>,

    pub(super) reward_period: &'a RewardPeriod,
    pub(super) rewards_table: &'a NodeRewardsTable,
    pub(super) daily_metrics_by_node: &'a BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    pub(super) _marker: PhantomData<T>,
}

impl<'a, T: ExecutionState> NodeProviderRewardsCalculator<'a, T> {
    fn transition<S: ExecutionState>(self) -> NodeProviderRewardsCalculator<'a, S> {
        NodeProviderRewardsCalculator {
            provider_id: self.provider_id,
            rewardable_nodes: self.rewardable_nodes,
            calculator_results: self.calculator_results,
            reward_period: self.reward_period,
            rewards_table: self.rewards_table,
            daily_metrics_by_node: self.daily_metrics_by_node,
            daily_subnets_fr: self.daily_subnets_fr,
            _marker: PhantomData,
        }
    }
}

impl<'a> NodeProviderRewardsCalculator<'a, Initialized> {
    pub(crate) fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputeExecutionNodes> {
        for node in self.rewardable_nodes.iter() {
            let node_results = self.calculator_results.nodes_results.entry(node.node_id).or_default();
            node_results.region = node.region.clone();
            node_results.node_type = node.node_type.clone();
        }
        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Calculates daily failure rates for a given set of `nodes` over the reward period.
///
/// If a node has no metrics recorded for a day, its failure rate is marked as [NodeFailureRate::Undefined].
/// Otherwise, it is recorded as [NodeFailureRate::Defined].
impl<'a> NodeProviderRewardsCalculator<'a, ComputeExecutionNodes> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputeRelativeFR> {
        let days_in_period = self.reward_period.days_between();
        let reward_period_start_ts = self.reward_period.start_ts.get();

        for (node_id, node_results) in self.calculator_results.nodes_results.iter_mut() {
            let metrics_in_period = Self::compute_metrics_in_period(days_in_period, reward_period_start_ts, node_id, self.daily_metrics_by_node);
            node_results.daily_metrics_processed = metrics_in_period;
        }

        NodeProviderRewardsCalculator::transition(self)
    }

    fn compute_metrics_in_period(
        days_in_period: u64,
        reward_period_start_ts: TimestampNanos,
        node_id: &NodeId,
        daily_metrics_by_node: &BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
    ) -> Vec<NodeMetricsDailyProcessed> {
        (0..days_in_period)
            .map(|day| {
                let ts = DayEndNanos::from(reward_period_start_ts + day * NANOS_PER_DAY);

                if let Some(metrics) = daily_metrics_by_node.get(node_id) {
                    let metrics_for_day = metrics.iter().filter(|m| m.ts == ts).collect_vec();

                    // Node is assigned in reward period but has no metrics for the day.
                    if metrics_for_day.is_empty() {
                        return NodeMetricsDailyProcessed::new_unassigned(ts);
                    }

                    // Node is assigned to only one subnet.
                    if metrics_for_day.len() == 1 {
                        let first_and_only = metrics_for_day.first().expect("Exists");

                        return NodeMetricsDailyProcessed::new_assigned(
                            ts,
                            first_and_only.subnet_assigned,
                            first_and_only.num_blocks_proposed,
                            first_and_only.num_blocks_failed,
                            first_and_only.failure_rate,
                        );
                    }

                    // Node is reassigned to different subnets on the same day.
                    // The algorithm considers for this case the subnet where the node has proposed and failed more blocks.
                    let metrics_max_blocks = metrics_for_day
                        .into_iter()
                        .max_by_key(|m| m.num_blocks_proposed + m.num_blocks_failed)
                        .expect("Exists");

                    NodeMetricsDailyProcessed::new_assigned(
                        ts,
                        metrics_max_blocks.subnet_assigned,
                        metrics_max_blocks.num_blocks_proposed,
                        metrics_max_blocks.num_blocks_failed,
                        metrics_max_blocks.failure_rate,
                    )
                } else {
                    NodeMetricsDailyProcessed::new_unassigned(ts)
                }
            })
            .collect_vec()
    }
}

/// Updates node failure rates to be relative to their subnet’s failure rate.
///
/// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
/// assigned to.
/// This is done for removing systematic factors that may affect all nodes in a subnet.
impl<'a> NodeProviderRewardsCalculator<'a, ComputeRelativeFR> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputeExtrapolatedFR> {
        for metrics in self
            .calculator_results
            .nodes_results
            .values_mut()
            .flat_map(|node_results| node_results.daily_metrics_processed.iter_mut())
        {
            if let (NodeStatus::Assigned { subnet_assigned, .. }, NodeFailureRate::Defined(value)) = (&metrics.status, &metrics.failure_rate) {
                let subnet_failure = self
                    .daily_subnets_fr
                    .get(subnet_assigned)
                    .and_then(|rates| rates.iter().find(|rate| rate.ts == metrics.ts))
                    .map(|rate| rate.value)
                    .expect("Subnet failure rate not found");

                let relative_failure = max(Decimal::ZERO, value - subnet_failure);

                metrics.failure_rate = NodeFailureRate::DefinedRelative(relative_failure);
            }
        }

        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Calculates the extrapolated failure rate used as replacement for nodes with `Undefined` failure rates.
///
/// For each node is computed the average of the relative failure rates recorded in the reward period.
/// The extrapolated failure rate is the average of these averages.
/// This is done to put higher weight on nodes with less recorded failure rates (assigned for fewer days).
impl<'a> NodeProviderRewardsCalculator<'a, ComputeExtrapolatedFR> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, FillUndefinedFR> {
        let mut nodes_avg_fr = Vec::new();
        for node_results in self.calculator_results.nodes_results.values_mut() {
            let failure_rates: Vec<Decimal> = node_results
                .daily_metrics_processed
                .iter()
                .filter_map(|entry| match entry.failure_rate {
                    NodeFailureRate::DefinedRelative(value) => Some(value),
                    _ => None,
                })
                .collect();

            // Do not consider nodes completely unassigned
            if !failure_rates.is_empty() {
                let node_avg_fr = avg(&failure_rates);
                node_results.average_relative_fr = node_avg_fr;
                nodes_avg_fr.push(node_avg_fr);
            }
        }
        self.calculator_results.extrapolated_fr = avg(&nodes_avg_fr);
        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Fills the `Undefined` failure rates with the extrapolated failure rate.
impl<'a> NodeProviderRewardsCalculator<'a, FillUndefinedFR> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputeAverageExtrapolatedFR> {
        for metrics in self
            .calculator_results
            .nodes_results
            .values_mut()
            .flat_map(|node_results| node_results.daily_metrics_processed.iter_mut())
        {
            if matches!(metrics.failure_rate, NodeFailureRate::Undefined) {
                metrics.failure_rate = NodeFailureRate::Extrapolated(self.calculator_results.extrapolated_fr);
            }
        }
        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Calculates the average of the failure rates (DefinedRelative and Extrapolated) for each node in the reward period.
///
/// The average failure rate is used to calculate the performance multiplier for each node.
impl<'a> NodeProviderRewardsCalculator<'a, ComputeAverageExtrapolatedFR> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputePerformanceMultipliers> {
        for (_, node_results) in self.calculator_results.nodes_results.iter_mut() {
            let raw_failure_rates: Vec<Decimal> = node_results
                .daily_metrics_processed
                .iter()
                .filter_map(|entry| match entry.failure_rate {
                    NodeFailureRate::DefinedRelative(value) | NodeFailureRate::Extrapolated(value) => Some(value),
                    _ => None,
                })
                .collect();

            node_results.average_extrapolated_fr = avg(&raw_failure_rates);
        }

        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Calculates the performance multiplier for a node based on its average failure rate.
impl<'a> NodeProviderRewardsCalculator<'a, ComputePerformanceMultipliers> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputeBaseRewardsByCategory> {
        for (_, node_results) in self.calculator_results.nodes_results.iter_mut() {
            let rewards_reduction;
            let average_failure_rate = node_results.average_extrapolated_fr;

            if average_failure_rate < MIN_FAILURE_RATE {
                rewards_reduction = MIN_REWARDS_REDUCTION;
            } else if average_failure_rate > MAX_FAILURE_RATE {
                rewards_reduction = MAX_REWARDS_REDUCTION;
            } else {
                // Linear interpolation between MIN_REWARDS_REDUCTION and MAX_REWARDS_REDUCTION
                rewards_reduction = ((average_failure_rate - MIN_FAILURE_RATE) / (MAX_FAILURE_RATE - MIN_FAILURE_RATE)) * MAX_REWARDS_REDUCTION;
            };

            node_results.rewards_reduction = rewards_reduction;
            node_results.performance_multiplier = dec!(1) - rewards_reduction;
        }

        NodeProviderRewardsCalculator::transition(self)
    }
}

struct Type3Rewards {
    coefficients: Vec<Decimal>,
    base_rewards: Vec<Decimal>,
}

fn is_type3(node_type: &str) -> bool {
    node_type.starts_with("type3")
}

fn type3_category(region: &str) -> NodeCategory {
    // The rewards table contains entries of this form DC Continent + DC Country + DC State/City.
    // The grouping for type3* nodes will be on DC Continent + DC Country level. This group is used for computing
    // the reduction coefficient and base reward for the group.
    let region_key = region.splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");
    NodeCategory {
        region: region_key,
        node_type: "type3*".to_string(),
    }
}

/// Calculate the base rewards for all the [NodeCategory].
///
/// The base rewards are calculated based on the rewards table entries for the specific region and node type.
/// For type3* nodes the base rewards are computed as the average of base rewards on DC Country level.
impl<'a> NodeProviderRewardsCalculator<'a, ComputeBaseRewardsByCategory> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, AdjustNodesRewards> {
        let mut type3_rewards_by_category: HashMap<NodeCategory, Type3Rewards> = HashMap::default();

        let nodes_count_by_category = self
            .rewardable_nodes
            .iter()
            .into_group_map_by(|node| NodeCategory {
                region: node.region.clone(),
                node_type: node.node_type.clone(),
            })
            .into_iter()
            .map(|(category, nodes)| (category, nodes.len()))
            .collect::<HashMap<_, _>>();

        for (category, nodes_count) in nodes_count_by_category {
            let (base_rewards, coefficient) = self
                .rewards_table
                .get_rate(&category.region, &category.node_type)
                .map(|rate| {
                    let base_rewards = Decimal::from(rate.xdr_permyriad_per_node_per_month);
                    // Default reward_coefficient_percent is set to 80%, which is used as a fallback only in the
                    // unlikely case that the type3 entry in the reward table:
                    // a) has xdr_permyriad_per_node_per_month entry set for this region, but
                    // b) does NOT have the reward_coefficient_percent value set
                    let reward_coefficient_percent = Decimal::from(rate.reward_coefficient_percent.unwrap_or(80)) / dec!(100);

                    (base_rewards, reward_coefficient_percent)
                })
                .unwrap_or((dec!(1), dec!(100)));

            // For nodes which are type3* the base rewards for the single node is computed as the average of base rewards
            // on DC Country level. Moreover, to de-stimulate the same NP having too many nodes in the same country,
            // the node rewards is reduced for each node the NP has in the given country. The reduction coefficient is
            // computed as the average of reduction coefficients on DC Country level.
            if is_type3(&category.node_type) && nodes_count > 0 {
                let coefficients = vec![coefficient; nodes_count];
                let base_rewards = vec![base_rewards; nodes_count];
                let type3_category = type3_category(&category.region);

                type3_rewards_by_category
                    .entry(type3_category)
                    .and_modify(|type3_rewards| {
                        type3_rewards.coefficients.extend(&coefficients);
                        type3_rewards.base_rewards.extend(&base_rewards);
                    })
                    .or_insert(Type3Rewards { coefficients, base_rewards });
            } else {
                // For `rewardable_nodes` which are not type3* the base rewards for the sigle node is the entry
                // in the rewards table for the specific region (DC Continent + DC Country + DC State/City) and node type.
                self.calculator_results.rewards_by_category.insert(category.clone(), base_rewards);
            }
        }

        // Computes node rewards for type3* nodes in all regions and add it to region_nodetype_rewards
        for (type3_category, type3_rewards) in type3_rewards_by_category {
            let rewards_len = type3_rewards.base_rewards.len();

            let coefficients_avg = avg(&type3_rewards.coefficients);
            let rewards_avg = avg(&type3_rewards.base_rewards);

            let mut running_coefficient = dec!(1);
            let mut region_rewards = Vec::new();
            for _ in 0..rewards_len {
                region_rewards.push(rewards_avg * running_coefficient);
                running_coefficient *= coefficients_avg;
            }
            let region_rewards_avg = avg(&region_rewards);
            self.calculator_results.rewards_by_category.insert(type3_category, region_rewards_avg);
        }
        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Calculate the adjusted rewards for all the nodes based on their performance.
impl<'a> NodeProviderRewardsCalculator<'a, AdjustNodesRewards> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, ComputeRewardsTotal> {
        let nodes_count = self.calculator_results.nodes_results.len() as u32;

        for node_results in self.calculator_results.nodes_results.values_mut() {
            let node_category = if is_type3(&node_results.node_type) {
                type3_category(&node_results.region)
            } else {
                NodeCategory {
                    region: node_results.region.clone(),
                    node_type: node_results.node_type.clone(),
                }
            };
            let base_rewards = *self
                .calculator_results
                .rewards_by_category
                .get(&node_category)
                .expect("Node category exist");

            if nodes_count <= FULL_REWARDS_MACHINES_LIMIT {
                // Node Providers with less than FULL_REWARDS_MACHINES_LIMIT machines are rewarded fully, independently of their performance

                node_results.adjusted_rewards = base_rewards;
            } else {
                node_results.adjusted_rewards = base_rewards * node_results.performance_multiplier;
            }
            node_results.base_rewards = base_rewards;
        }

        NodeProviderRewardsCalculator::transition(self)
    }
}

/// Calculate the adjusted rewards for all the nodes based on their performance.
impl<'a> NodeProviderRewardsCalculator<'a, ComputeRewardsTotal> {
    pub fn next(mut self) -> NodeProviderRewardsCalculator<'a, RewardsTotalComputed> {
        let rewards_total = self
            .calculator_results
            .nodes_results
            .values()
            .map(|node_results| node_results.adjusted_rewards)
            .sum::<Decimal>();

        self.calculator_results.rewards_total = rewards_total;
        NodeProviderRewardsCalculator::transition(self)
    }
}

impl NodeProviderRewardsCalculator<'_, RewardsTotalComputed> {
    pub fn get_results(self) -> RewardsCalculatorResults {
        let mut results = self.calculator_results;

        // Add the daily subnets failure rates to the results
        results.daily_subnets_fr = self.daily_subnets_fr;
        results
    }
}

pub trait ExecutionState {}

pub(crate) struct Initialized;
impl ExecutionState for Initialized {}
pub(crate) struct ComputeExecutionNodes;
impl ExecutionState for ComputeExecutionNodes {}
pub(crate) struct ComputeRelativeFR;
impl ExecutionState for ComputeRelativeFR {}
pub(crate) struct ComputeExtrapolatedFR;
impl ExecutionState for ComputeExtrapolatedFR {}
pub(crate) struct FillUndefinedFR {}
impl ExecutionState for FillUndefinedFR {}
pub(crate) struct ComputeAverageExtrapolatedFR;
impl ExecutionState for ComputeAverageExtrapolatedFR {}
pub(crate) struct ComputePerformanceMultipliers;
impl ExecutionState for ComputePerformanceMultipliers {}
pub(crate) struct ComputeBaseRewardsByCategory;
impl ExecutionState for ComputeBaseRewardsByCategory {}
pub(crate) struct AdjustNodesRewards;
impl ExecutionState for AdjustNodesRewards {}
pub(crate) struct ComputeRewardsTotal;
impl ExecutionState for ComputeRewardsTotal {}

pub(crate) struct RewardsTotalComputed;
impl ExecutionState for RewardsTotalComputed {}

#[cfg(test)]
mod tests;
