use crate::logs::{Logger, Operation};
use crate::rewardable_nodes::{RegionNodeTypeCategory, RewardableNode};
use ic_base_types::NodeId;
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardsTable};
use num_traits::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::{RefCell, RefMut};
use std::collections::{BTreeMap, HashMap};

const FULL_REWARDS_MACHINES_LIMIT: u32 = 4;
pub type XDRPermyriad = u64;
pub struct RewardsComputationResult {
    pub logger: Logger,
    pub xdr_permyriad: XDRPermyriad,
}

struct Type3Rewards {
    coefficients: Vec<Decimal>,
    base_rewards: Vec<Decimal>,
}
struct ExecutionContext {
    logger: RefCell<Logger>,
}
pub struct RewardsCalculator {
    rewards_table: NodeRewardsTable,
    ctx: ExecutionContext,
}

impl RewardsCalculator {
    pub fn new(rewards_table: NodeRewardsTable) -> Self {
        Self {
            rewards_table,
            ctx: ExecutionContext {
                logger: RefCell::new(Logger::default()),
            },
        }
    }
    fn logger_mut(&self) -> RefMut<Logger> {
        self.ctx.logger.borrow_mut()
    }

    fn take_ctx(&self) -> Logger {
        self.ctx.logger.replace(Logger::default())
    }

    fn type3_category(&self, region: &str) -> RegionNodeTypeCategory {
        // The rewards table contains entries of this form DC Continent + DC Country + DC State/City.
        // The grouping for type3* nodes will be on DC Continent + DC Country level. This group is used for computing
        // the reduction coefficient and base reward for the group.
        let region_key = region.splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");
        RegionNodeTypeCategory {
            region: region_key,
            node_type: "type3*".to_string(),
        }
    }

    /// Calculate the base rewards for all the regions and node types.
    ///
    /// The base rewards are calculated based on the rewards table entries for the specific region and node type.
    /// For type3* nodes the base rewards are computed as the average of base rewards on DC Country level.
    fn base_rewards_by_category(&self, nodes_count_by_category: HashMap<RegionNodeTypeCategory, u32>) -> HashMap<RegionNodeTypeCategory, Decimal> {
        let mut type3_rewards_by_category: HashMap<RegionNodeTypeCategory, Type3Rewards> = HashMap::default();
        let mut rewards_by_category: HashMap<RegionNodeTypeCategory, Decimal> = HashMap::default();

        for (category, node_count) in nodes_count_by_category {
            let rate = self
                .rewards_table
                .get_rate(&category.region, &category.node_type)
                .unwrap_or(NodeRewardRate {
                    xdr_permyriad_per_node_per_month: 1,
                    reward_coefficient_percent: Some(100),
                });
            let base_rewards = Decimal::from(rate.xdr_permyriad_per_node_per_month);
            let mut coeff = dec!(1);

            if category.node_type.starts_with("type3") && node_count > 0 {
                // For nodes which are type3* the base rewards for the single node is computed as the average of base rewards
                // on DC Country level. Moreover, to de-stimulate the same NP having too many nodes in the same country,
                // the node rewards is reduced for each node the NP has in the given country. The reduction coefficient is
                // computed as the average of reduction coefficients on DC Country level.

                coeff = Decimal::from(rate.reward_coefficient_percent.unwrap_or(80)) / dec!(100);
                let coefficients = vec![coeff; node_count as usize];
                let base_rewards = vec![base_rewards; node_count as usize];
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
            let mut running_coefficient = dec!(1);
            let mut region_rewards = Vec::new();

            let coefficients_avg = self
                .logger_mut()
                .run_and_log("Coefficients avg.", Operation::Avg(type3_rewards.coefficients));
            let rewards_avg = self.logger_mut().run_and_log("Rewards avg.", Operation::Avg(type3_rewards.base_rewards));
            for _ in 0..rewards_len {
                region_rewards.push(Operation::Multiply(rewards_avg, running_coefficient));
                running_coefficient *= coefficients_avg;
            }
            let region_rewards = self
                .logger_mut()
                .run_and_log("Total rewards after coefficient reduction", Operation::SumOps(region_rewards));
            let region_rewards_avg = self.logger_mut().run_and_log(
                "Rewards average after coefficient reduction",
                Operation::Divide(region_rewards, Decimal::from(rewards_len)),
            );

            rewards_by_category.insert(type3_category, region_rewards_avg);
        }

        rewards_by_category
    }

    /// Calculate rewards in XDR for the given `rewardable_nodes` adjusted based on their performances.
    pub fn calculate_rewards_xdr(
        &self,
        rewardable_nodes: Vec<RewardableNode>,
        performance_multiplier_by_node: BTreeMap<NodeId, Decimal>,
    ) -> RewardsComputationResult {
        let mut rewards_total = Vec::new();
        let mut performance_multiplier_by_node = performance_multiplier_by_node;

        // 1. calculate the base rewards for all the `RegionNodeTypeCategory`
        let mut nodes_count_by_category = HashMap::default();
        for node in rewardable_nodes.iter() {
            let count = nodes_count_by_category.entry(node.region_node_type_category()).or_default();
            *count += 1;
        }
        let base_rewards_by_category: HashMap<RegionNodeTypeCategory, Decimal> = self.base_rewards_by_category(nodes_count_by_category);

        let nodes_count = rewardable_nodes.len() as u32;
        let mut nodes_sorted = rewardable_nodes;
        nodes_sorted.sort_by(|a, b| a.region.cmp(&b.region).then(a.node_type.cmp(&b.node_type)));
        for node in nodes_sorted {
            let category = if node.node_type.starts_with("type3") {
                self.type3_category(&node.region)
            } else {
                node.region_node_type_category()
            };

            let base_rewards = base_rewards_by_category.get(&category).expect("Categories filled already");

            // Node Providers with less than FULL_REWARDS_MACHINES_LIMIT machines are rewarded fully, independently of their performance
            if nodes_count <= FULL_REWARDS_MACHINES_LIMIT {
                rewards_total.push(*base_rewards);
                continue;
            }

            let performance_multiplier = performance_multiplier_by_node.remove(&node.node_id).expect("Rewards multiplier exist");
            let rewards_adjusted = self.logger_mut().run_and_log(
                "Rewards permyriad XDR for the node",
                Operation::Multiply(*base_rewards, performance_multiplier),
            );
            rewards_total.push(rewards_adjusted);
        }

        let rewards_total = self
            .logger_mut()
            .run_and_log("Total rewards for all nodes", Operation::Sum(rewards_total));

        RewardsComputationResult {
            logger: self.take_ctx(),
            xdr_permyriad: rewards_total.to_u64().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests;
