use std::collections::BTreeMap;
use ic_base_types::NodeId;
use crate::reward_period::RewardPeriod;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use rust_decimal::Decimal;
use crate::performance_calculator::PerformanceMultipliers;
use crate::rewardable_nodes::RewardableNode;

pub struct RewardsCalculator {
    reward_period: RewardPeriod,
    rewards_table: NodeRewardsTable,
}

impl RewardsCalculator {
    pub fn new(reward_period: RewardPeriod, rewards_table: NodeRewardsTable) -> Self {
        Self {
            reward_period,
            rewards_table,
        }
    }

    pub fn calculate_rewards_xdr(&self, provider_rewardable_nodes: Vec<RewardableNode>, nodes_multiplier: BTreeMap<NodeId, Decimal>) {
        
        let rewardable_nodes_count = rewardables.len() as u32;
        let mut nodes_idiosyncratic_fr = nodes_idiosyncratic_fr;

        // 0. Compute base rewards for each region and node type
        logger.add_entry(
            LogLevel::High,
            LogEntry::ComputeBaseRewardsForRegionNodeType,
        );
        for node in rewardables {
            let nodes_count = region_node_type_rewardables
                .entry((node.region.clone(), node.node_type.clone()))
                .or_default();
            *nodes_count += 1;
        }
        let region_nodetype_rewards: HashMap<RegionNodeTypeCategory, Decimal> =
            base_rewards_region_nodetype(logger, &region_node_type_rewardables, rewards_table);

        // 3. reward the nodes of node provider
        let mut sorted_rewardables = rewardables.to_vec();
        sorted_rewardables.sort_by(|a, b| a.region.cmp(&b.region).then(a.node_type.cmp(&b.node_type)));
        for node in sorted_rewardables {
            logger.add_entry(
                LogLevel::High,
                LogEntry::ComputeRewardsForNode {
                    node_id: node.node_id,
                    node_type: node.node_type.clone(),
                    region: node.region.clone(),
                },
            );

            let node_type = node.node_type.clone();
            let region = node.region.clone();

            let rewards_xdr_no_penalty = if node_type.starts_with("type3") {
                let region_key = region_type3_key(region.clone());
                region_nodetype_rewards
                    .get(&region_key)
                    .expect("Type3 rewards already filled")
            } else {
                region_nodetype_rewards
                    .get(&(node.region.clone(), node.node_type.clone()))
                    .expect("Rewards already filled")
            };

            logger.add_entry(
                LogLevel::Mid,
                LogEntry::BaseRewards(*rewards_xdr_no_penalty),
            );

            rewards_xdr_no_penalty_total.push(*rewards_xdr_no_penalty);

            // Node Providers with less than 4 machines are rewarded fully, independently of their performance
            if rewardable_nodes_count <= FULL_REWARDS_MACHINES_LIMIT {
                rewards_xdr_total.push(*rewards_xdr_no_penalty);
                continue;
            }

            let reward_multiplier = // get it from the performance calculator

            let rewards_xdr = logger.execute(
                "Rewards XDR for the node",
                Operation::Multiply(*rewards_xdr_no_penalty, reward_multiplier),
            );
            rewards_xdr_total.push(rewards_xdr);
        }

        Rewards {
            xdr_permyriad: rewards_xdr_total.to_u64().unwrap(),
            xdr_permyriad_no_reduction: rewards_xdr_no_reduction_total.to_u64().unwrap(),
        }
    }
    }
}
