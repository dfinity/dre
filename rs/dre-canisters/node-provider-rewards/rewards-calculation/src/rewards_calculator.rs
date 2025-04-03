use crate::calculation_results::{NodeCategory, RewardsCalculatorResults};
use crate::input_builder::{NodeDailyFailureRate, NodeFailureRate, NodeMetricsDaily, RewardableNode, RewardsCalculatorInput};
use crate::types::{DayEndNanos, NANOS_PER_DAY};
use ic_base_types::SubnetId;
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::max;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

/// The minimum and maximum failure rates for a node.
/// Nodes with a failure rate below `MIN_FAILURE_RATE` will not be penalized.
/// Nodes with a failure rate above `MAX_FAILURE_RATE` will be penalized with `MAX_REWARDS_REDUCTION`.
pub const MIN_FAILURE_RATE: Decimal = dec!(0.1);
pub const MAX_FAILURE_RATE: Decimal = dec!(0.6);

/// The minimum and maximum rewards reduction for a node.
pub const MIN_REWARDS_REDUCTION: Decimal = dec!(0);
pub const MAX_REWARDS_REDUCTION: Decimal = dec!(0.8);

const FULL_REWARDS_MACHINES_LIMIT: u32 = 4;

pub fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}

pub(crate) struct RewardsCalculator<'a, T: ExecutionState> {
    pub(crate) input: &'a RewardsCalculatorInput,
    pub(crate) rewardable_nodes: Vec<RewardableNode>,
    pub(crate) calculator_results: RewardsCalculatorResults,
    pub(crate) _marker: PhantomData<T>,
}

impl<'a, T: ExecutionState> RewardsCalculator<'a, T> {
    fn transition<S: ExecutionState>(self) -> RewardsCalculator<'a, S> {
        RewardsCalculator {
            input: self.input,
            rewardable_nodes: self.rewardable_nodes,
            calculator_results: self.calculator_results,
            _marker: PhantomData,
        }
    }
}

impl<'a> RewardsCalculator<'a, Initialized> {
    pub(crate) fn next(mut self) -> RewardsCalculator<'a, FillDailyMetrics> {
        for node in self.rewardable_nodes.iter() {
            let node_results = self.calculator_results.nodes_results.entry(node.node_id).or_default();
            node_results.region = node.region.clone();
            node_results.node_type = node.node_type.clone();
        }
        RewardsCalculator::transition(self)
    }
}

impl<'a> RewardsCalculator<'a, FillDailyMetrics> {
    pub(crate) fn next(mut self) -> RewardsCalculator<'a, ComputeExecutionNodesFR> {
        for (node_id, node_results) in self.calculator_results.nodes_results.iter_mut() {
            node_results.daily_metrics = self.input.daily_metrics_by_node.get(node_id).cloned().unwrap_or_default();
        }
        RewardsCalculator::transition(self)
    }
}

/// Calculates daily failure rates for a given set of `nodes` over the reward period.
///
/// If a node has no metrics recorded for a day, its failure rate is marked as [NodeFailureRate::Undefined].
/// Otherwise, it is recorded as [NodeFailureRate::Defined].
impl<'a> RewardsCalculator<'a, ComputeExecutionNodesFR> {
    fn one_day_node_fr(one_day_metrics: Vec<&NodeMetricsDaily>) -> NodeFailureRate {
        // Node is assigned in reward period but has no metrics for the day.
        if one_day_metrics.is_empty() {
            return NodeFailureRate::Undefined;
        };

        // Node is assigned to only one subnet.
        if one_day_metrics.len() == 1 {
            let first = one_day_metrics.first().expect("No metrics");

            return NodeFailureRate::Defined {
                subnet_assigned: first.subnet_assigned,
                value: first.failure_rate,
            };
        }

        // Node is reassigned to different subnets on the same day.
        // The algorithm considers for this case the subnet where the node has proposed and failed more blocks.
        let mut subnet_block_counts: BTreeMap<SubnetId, u64> = BTreeMap::new();

        for metrics in one_day_metrics.iter() {
            let all_blocks = metrics.num_blocks_proposed + metrics.num_blocks_failed;
            *subnet_block_counts.entry(metrics.subnet_assigned).or_insert(0) += all_blocks;
        }

        let (subnet_assigned, _) = subnet_block_counts.into_iter().max_by_key(|&(_, count)| count).expect("No subnet found");

        let failure_rate = one_day_metrics
            .iter()
            .find(|m| m.subnet_assigned == subnet_assigned)
            .expect("No metrics for the selected subnet")
            .failure_rate;

        NodeFailureRate::Defined {
            subnet_assigned,
            value: failure_rate,
        }
    }
    pub fn next(mut self) -> RewardsCalculator<'a, ComputeRelativeFR> {
        let days_in_period = self.input.reward_period.days_between();
        let reward_period_start_ts = self.input.reward_period.start_ts.get();

        for (node_id, node_results) in self.calculator_results.nodes_results.iter_mut() {
            let failure_rates_in_period = (0..days_in_period)
                .map(|day| {
                    let ts = DayEndNanos::from(reward_period_start_ts + day * NANOS_PER_DAY);

                    let value = match self.input.daily_metrics_by_node.get(node_id) {
                        Some(metrics) => {
                            let metrics_for_day = metrics.iter().filter(|m| m.ts == ts).collect_vec();
                            Self::one_day_node_fr(metrics_for_day)
                        }
                        None => NodeFailureRate::Undefined,
                    };
                    NodeDailyFailureRate { ts, value }
                })
                .collect_vec();

            node_results.daily_fr = failure_rates_in_period;
        }

        RewardsCalculator::transition(self)
    }
}

/// Updates node failure rates to be relative to their subnet’s failure rate.
///
/// Defined failure rates are adjusted by discounting the failure rate of the subnet to which they are
/// assigned to.
/// This is done for removing systematic factors that may affect all nodes in a subnet.
impl<'a> RewardsCalculator<'a, ComputeRelativeFR> {
    pub fn next(mut self) -> RewardsCalculator<'a, ComputeExtrapolatedFR> {
        for failure_rate in self
            .calculator_results
            .nodes_results
            .values_mut()
            .flat_map(|node_results| node_results.daily_fr.iter_mut())
        {
            if let NodeFailureRate::Defined { subnet_assigned, value } = failure_rate.value {
                let subnet_failure = self
                    .input
                    .daily_subnets_fr
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

        RewardsCalculator::transition(self)
    }
}

/// Calculates the extrapolated failure rate used as replacement for nodes with `Undefined` failure rates.
///
/// For each node is computed the average of the relative failure rates recorded in the reward period.
/// The extrapolated failure rate is the average of these averages.
/// This is done to put higher weight on nodes with less recorded failure rates (assigned for fewer days).
impl<'a> RewardsCalculator<'a, ComputeExtrapolatedFR> {
    pub fn next(mut self) -> RewardsCalculator<'a, FillUndefinedFR> {
        if self.calculator_results.nodes_results.is_empty() {
            self.calculator_results.extrapolated_fr = dec!(1);
            return RewardsCalculator::transition(self);
        }

        let mut nodes_avg_fr = Vec::new();
        for node_results in self.calculator_results.nodes_results.values_mut() {
            let failure_rates: Vec<Decimal> = node_results
                .daily_fr
                .iter()
                .filter_map(|entry| match entry.value {
                    NodeFailureRate::DefinedRelative { value, .. } => Some(value),
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
        RewardsCalculator::transition(self)
    }
}

/// Fills the `Undefined` failure rates with the extrapolated failure rate.
impl<'a> RewardsCalculator<'a, FillUndefinedFR> {
    pub fn next(mut self) -> RewardsCalculator<'a, ComputeAverageExtrapolatedFR> {
        for failure_rate in self
            .calculator_results
            .nodes_results
            .values_mut()
            .flat_map(|node_results| node_results.daily_fr.iter_mut())
        {
            if matches!(failure_rate.value, NodeFailureRate::Undefined) {
                failure_rate.value = NodeFailureRate::Extrapolated(self.calculator_results.extrapolated_fr);
            }
        }
        RewardsCalculator::transition(self)
    }
}

/// Calculates the average of the failure rates (DefinedRelative and Extrapolated) for each node in the reward period.
///
/// The average failure rate is used to calculate the performance multiplier for each node.
impl<'a> RewardsCalculator<'a, ComputeAverageExtrapolatedFR> {
    pub fn next(mut self) -> RewardsCalculator<'a, ComputePerformanceMultipliers> {
        for (_, node_results) in self.calculator_results.nodes_results.iter_mut() {
            let raw_failure_rates: Vec<Decimal> = node_results
                .daily_fr
                .iter()
                .filter_map(|entry| match entry.value {
                    NodeFailureRate::DefinedRelative { value, .. } | NodeFailureRate::Extrapolated(value) => Some(value),
                    _ => None,
                })
                .collect();

            node_results.average_extrapolated_fr = avg(&raw_failure_rates);
        }

        RewardsCalculator::transition(self)
    }
}

/// Calculates the performance multiplier for a node based on its average failure rate.
impl<'a> RewardsCalculator<'a, ComputePerformanceMultipliers> {
    pub fn next(mut self) -> RewardsCalculator<'a, ComputeBaseRewardsByCategory> {
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

        RewardsCalculator::transition(self)
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
impl<'a> RewardsCalculator<'a, ComputeBaseRewardsByCategory> {
    pub fn next(mut self) -> RewardsCalculator<'a, AdjustNodesRewards> {
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
                .input
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
        RewardsCalculator::transition(self)
    }
}

/// Calculate the adjusted rewards for all the nodes based on their performance.
impl<'a> RewardsCalculator<'a, AdjustNodesRewards> {
    pub fn next(mut self) -> RewardsCalculator<'a, ComputeRewardsTotal> {
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

        RewardsCalculator::transition(self)
    }
}

/// Calculate the adjusted rewards for all the nodes based on their performance.
impl<'a> RewardsCalculator<'a, ComputeRewardsTotal> {
    pub fn next(mut self) -> RewardsCalculator<'a, RewardsTotalComputed> {
        let rewards_total = self
            .calculator_results
            .nodes_results
            .values()
            .map(|node_results| node_results.adjusted_rewards)
            .sum::<Decimal>();

        self.calculator_results.rewards_total = rewards_total;
        RewardsCalculator::transition(self)
    }
}

impl<'a> RewardsCalculator<'a, RewardsTotalComputed> {
    pub fn get_results(self) -> RewardsCalculatorResults {
        self.calculator_results
    }
}

pub trait ExecutionState {}

pub(crate) struct Initialized;
impl ExecutionState for Initialized {}
pub(crate) struct FillDailyMetrics;
impl ExecutionState for FillDailyMetrics {}
pub(crate) struct ComputeExecutionNodesFR;
impl ExecutionState for ComputeExecutionNodesFR {}
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
