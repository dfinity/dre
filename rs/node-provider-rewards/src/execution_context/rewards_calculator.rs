use crate::execution_context::results_tracker::{NodeCategoryResult, NodeResult, ResultsTracker, SingleResult};
use crate::execution_context::{avg, ExecutionState, RewardsTotalComputed};
use crate::types::{NodeCategory, RewardableNode};
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::marker::PhantomData;

const FULL_REWARDS_MACHINES_LIMIT: u32 = 4;
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

fn nodes_count_by_category(rewardable_nodes: &[RewardableNode]) -> HashMap<NodeCategory, usize> {
    let mut nodes_count_by_category: HashMap<NodeCategory, usize> = HashMap::new();

    for node in rewardable_nodes.iter() {
        nodes_count_by_category
            .entry(node.category())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    nodes_count_by_category
}

pub(super) struct RewardsCalculatorContext<'a, T: ExecutionState> {
    pub(super) rewards_table: &'a NodeRewardsTable,
    pub(super) provider_nodes: Vec<RewardableNode>,
    pub(super) results_tracker: ResultsTracker,
    pub(super) _marker: PhantomData<T>,
}

impl<'a, T: ExecutionState> RewardsCalculatorContext<'a, T> {
    pub fn transition<S: ExecutionState>(self) -> RewardsCalculatorContext<'a, S> {
        RewardsCalculatorContext {
            rewards_table: self.rewards_table,
            provider_nodes: self.provider_nodes,
            results_tracker: self.results_tracker,
            _marker: PhantomData,
        }
    }
}

impl<'a> RewardsCalculatorContext<'a, StartRewardsCalculator> {
    pub fn next(self) -> RewardsCalculatorContext<'a, ComputeBaseRewardsByCategory> {
        RewardsCalculatorContext::transition(self)
    }
}

impl<'a> RewardsCalculatorContext<'a, ComputeBaseRewardsByCategory> {
    /// Calculate the base rewards for all the [NodeCategory].
    ///
    /// The base rewards are calculated based on the rewards table entries for the specific region and node type.
    /// For type3* nodes the base rewards are computed as the average of base rewards on DC Country level.
    pub fn next(mut self) -> RewardsCalculatorContext<'a, AdjustNodesRewards> {
        let mut type3_rewards_by_category: HashMap<NodeCategory, Type3Rewards> = HashMap::default();

        let nodes_count_by_category = nodes_count_by_category(&self.provider_nodes);
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
                self.results_tracker
                    .record_category_result(NodeCategoryResult::RewardsByCategory, &category, &base_rewards);
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
            self.results_tracker
                .record_category_result(NodeCategoryResult::RewardsByCategory, &type3_category, &region_rewards_avg);
        }
        RewardsCalculatorContext::transition(self)
    }
}

impl<'a> RewardsCalculatorContext<'a, AdjustNodesRewards> {
    /// Calculate the adjusted rewards for all the nodes based on their performance.
    pub fn next(mut self) -> RewardsCalculatorContext<'a, ComputeRewardsTotal> {
        let nodes_count = self.provider_nodes.len() as u32;

        let sorted_nodes = self
            .provider_nodes
            .iter()
            .sorted_by(|a, b| a.region.cmp(&b.region).then(a.node_type.cmp(&b.node_type)))
            .collect::<Vec<_>>();

        for node in sorted_nodes {
            let performance_multipliers = self.results_tracker.get_nodes_result(NodeResult::PerformanceMultiplier);
            let base_rewards_by_category = self.results_tracker.get_category_result(NodeCategoryResult::RewardsByCategory);

            let node_category = if is_type3(&node.node_type) {
                type3_category(&node.region)
            } else {
                node.category()
            };
            let base_rewards = *base_rewards_by_category.get(&node_category).expect("Node category exist");
            let node_performance_multiplier = performance_multipliers.get(&node.node_id).expect("Rewards multiplier exist");

            if nodes_count <= FULL_REWARDS_MACHINES_LIMIT {
                // Node Providers with less than FULL_REWARDS_MACHINES_LIMIT machines are rewarded fully, independently of their performance

                self.results_tracker
                    .record_node_result(NodeResult::AdjustedRewards, &node.node_id, &base_rewards);
            } else {
                let adjusted_rewards = base_rewards * node_performance_multiplier;
                self.results_tracker
                    .record_node_result(NodeResult::AdjustedRewards, &node.node_id, &adjusted_rewards);
            }
            self.results_tracker
                .record_node_result(NodeResult::BaseRewards, &node.node_id, &base_rewards);
        }

        RewardsCalculatorContext::transition(self)
    }
}

impl<'a> RewardsCalculatorContext<'a, ComputeRewardsTotal> {
    /// Calculate the adjusted rewards for all the nodes based on their performance.
    pub fn next(mut self) -> RewardsCalculatorContext<'a, RewardsTotalComputed> {
        let adjusted_rewards_by_node = self.results_tracker.get_nodes_result(NodeResult::AdjustedRewards);

        let rewards_total = adjusted_rewards_by_node.iter().map(|(_, reward)| *reward).sum::<Decimal>();

        self.results_tracker.record_single_result(SingleResult::RewardsTotal, &rewards_total);
        RewardsCalculatorContext::transition(self)
    }
}

pub(super) struct StartRewardsCalculator;
impl ExecutionState for StartRewardsCalculator {}
pub(super) struct ComputeBaseRewardsByCategory;
impl ExecutionState for ComputeBaseRewardsByCategory {}
pub(super) struct AdjustNodesRewards;
impl ExecutionState for AdjustNodesRewards {}
pub(super) struct ComputeRewardsTotal;
impl ExecutionState for ComputeRewardsTotal {}
