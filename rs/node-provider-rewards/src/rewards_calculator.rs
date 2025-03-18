use crate::execution_context::{ComputedPerformanceMultiplier, ExecutionContext, RewardsTotalComputed};
use crate::intermediate_results::{AllNodesResult, SingleNodeResult};
use crate::npr_utils::{avg, NodeCategory, RewardableNode};
use ic_base_types::NodeId;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use itertools::Itertools;
use num_traits::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, HashMap};

const FULL_REWARDS_MACHINES_LIMIT: u32 = 4;
struct Type3Rewards {
    coefficients: Vec<Decimal>,
    base_rewards: Vec<Decimal>,
}

pub struct RewardsCalculator<'a> {
    rewards_table: &'a NodeRewardsTable,
}

impl<'a> RewardsCalculator<'a> {
    pub fn new(rewards_table: &'a NodeRewardsTable) -> Self {
        Self { rewards_table }
    }

    fn is_type3(&self, node_type: &str) -> bool {
        node_type.starts_with("type3")
    }

    fn type3_category(&self, region: &str) -> NodeCategory {
        // The rewards table contains entries of this form DC Continent + DC Country + DC State/City.
        // The grouping for type3* nodes will be on DC Continent + DC Country level. This group is used for computing
        // the reduction coefficient and base reward for the group.
        let region_key = region.splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");
        NodeCategory {
            region: region_key,
            node_type: "type3*".to_string(),
        }
    }

    fn nodes_count_by_category(&self, rewardable_nodes: &[RewardableNode]) -> HashMap<NodeCategory, usize> {
        let mut nodes_count_by_category: HashMap<NodeCategory, usize> = HashMap::new();

        for node in rewardable_nodes.iter() {
            nodes_count_by_category
                .entry(node.category())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        nodes_count_by_category
    }

    /// Calculate the base rewards for all the [NodeCategory].
    ///
    /// The base rewards are calculated based on the rewards table entries for the specific region and node type.
    /// For type3* nodes the base rewards are computed as the average of base rewards on DC Country level.
    fn base_rewards_by_category(&self, ctx: &mut ExecutionContext<ComputedPerformanceMultiplier>) -> HashMap<NodeCategory, Decimal> {
        let mut type3_rewards_by_category: HashMap<NodeCategory, Type3Rewards> = HashMap::default();
        let mut rewards_by_category: HashMap<NodeCategory, Decimal> = HashMap::default();

        let nodes_count_by_category = self.nodes_count_by_category(&ctx.provider_nodes);
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
            if self.is_type3(&category.node_type) && nodes_count > 0 {
                let coefficients = vec![coefficient; nodes_count];
                let base_rewards = vec![base_rewards; nodes_count];
                let type3_category = self.type3_category(&category.region);

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
                rewards_by_category.insert(category, base_rewards);
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
            rewards_by_category.insert(type3_category, region_rewards_avg);
        }

        rewards_by_category
    }

    /// Calculate the adjusted rewards for all the nodes based on their performance.
    fn adjusted_rewards_by_node(
        &self,
        ctx: &mut ExecutionContext<ComputedPerformanceMultiplier>,
        base_rewards_by_category: HashMap<NodeCategory, Decimal>,
    ) -> BTreeMap<NodeId, Decimal> {
        let nodes_count = ctx.provider_nodes.len() as u32;

        ctx.provider_nodes
            .clone()
            .into_iter()
            .sorted_by(|a, b| a.region.cmp(&b.region).then(a.node_type.cmp(&b.node_type)))
            .map(|node| {
                let node_category = if self.is_type3(&node.node_type) {
                    self.type3_category(&node.region)
                } else {
                    node.category()
                };
                let base_rewards = base_rewards_by_category.get(&node_category).expect("Node category exist");
                ctx.results_tracker
                    .record_node_result(SingleNodeResult::BaseRewards, &node.node_id, base_rewards);

                if nodes_count <= FULL_REWARDS_MACHINES_LIMIT {
                    // Node Providers with less than FULL_REWARDS_MACHINES_LIMIT machines are rewarded fully, independently of their performance

                    ctx.results_tracker
                        .record_node_result(SingleNodeResult::AdjustedRewards, &node.node_id, base_rewards);
                    (node.node_id, *base_rewards)
                } else {
                    let performance_multiplier = ctx
                        .performance_multiplier_by_node
                        .get(&node.node_id)
                        .expect("Rewards multiplier exist");
                    
                    let adjusted_rewards = *base_rewards * performance_multiplier;
                    ctx.results_tracker
                        .record_node_result(SingleNodeResult::AdjustedRewards, &node.node_id, &adjusted_rewards);
                    (node.node_id, adjusted_rewards)
                }
            })
            .collect()
    }

    fn calculate_rewards_total(
        &self,
        ctx: ExecutionContext<ComputedPerformanceMultiplier>,
        adjusted_rewards_by_node: BTreeMap<NodeId, Decimal>,
    ) -> ExecutionContext<RewardsTotalComputed> {
        let mut ctx = ctx;

        let adjusted_rewards: Vec<Decimal> = adjusted_rewards_by_node.into_values().collect();
        let rewards_total = adjusted_rewards.iter().sum::<Decimal>();

        ctx.results_tracker.record_all_nodes_result(AllNodesResult::RewardsTotal, &rewards_total);
        ctx.rewards_total = rewards_total.to_u64().expect("Rewards total is u64");
        ctx.next()
    }

    /// Calculate rewards in XDR for the given `rewardable_nodes` adjusted based on their performances.
    pub fn calculate(&self, ctx: ExecutionContext<ComputedPerformanceMultiplier>) -> ExecutionContext<RewardsTotalComputed> {
        let mut ctx = ctx;
        let base_rewards_by_category = self.base_rewards_by_category(&mut ctx);
        let adjusted_rewards_by_node = self.adjusted_rewards_by_node(&mut ctx, base_rewards_by_category);

        self.calculate_rewards_total(ctx, adjusted_rewards_by_node)
    }
}
