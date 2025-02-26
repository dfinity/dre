use ic_base_types::{NodeId, PrincipalId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct RewardableNode {
    pub node_id: NodeId,
    pub node_provider: PrincipalId,
    pub region: String,
    pub node_type: String,
}

pub fn rewardable_nodes_by_provider(rewardable_nodes: &[RewardableNode]) -> BTreeMap<PrincipalId, Vec<RewardableNode>> {
    let mut nodes_by_provider: BTreeMap<PrincipalId, Vec<RewardableNode>> = BTreeMap::new();

    for node in rewardable_nodes {
        nodes_by_provider.entry(node.node_provider).or_default().push(node.clone());
    }
    nodes_by_provider
}

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}

fn region_type3_key(region: String) -> RegionNodeTypeCategory {
    // The rewards table contains entries of this form DC Continent + DC Country + DC State/City.
    // The grouping for type3* nodes will be on DC Continent + DC Country level. This group is used for computing
    // the reduction coefficient and base reward for the group.

    let region_key = region.splitn(3, ',').take(2).collect::<Vec<&str>>().join(":");
    (region_key, "type3*".to_string())
}

fn base_rewards_region_nodetype(
    logger: &mut RewardsLog,
    rewardable_nodes: &HashMap<RegionNodeTypeCategory, u32>,
    rewards_table: &NodeRewardsTable,
) -> HashMap<RegionNodeTypeCategory, Decimal> {
    let mut type3_coefficients_rewards: HashMap<RegionNodeTypeCategory, (Vec<Decimal>, Vec<Decimal>)> = HashMap::default();
    let mut region_nodetype_rewards: HashMap<RegionNodeTypeCategory, Decimal> = HashMap::default();

    for ((region, node_type), node_count) in rewardable_nodes {
        let rate = match rewards_table.get_rate(region, node_type) {
            Some(rate) => rate,
            None => {
                logger.add_entry(
                    LogLevel::High,
                    LogEntry::RateNotFoundInRewardTable {
                        node_type: node_type.to_string(),
                        region: region.to_string(),
                    },
                );

                NodeRewardRate {
                    xdr_permyriad_per_node_per_month: 1,
                    reward_coefficient_percent: Some(100),
                }
            }
        };
        let base_rewards = Decimal::from(rate.xdr_permyriad_per_node_per_month);
        let mut coeff = dec!(1);

        if node_type.starts_with("type3") && *node_count > 0 {
            // For nodes which are type3* the base rewards for the single node is computed as the average of base rewards
            // on DC Country level. Moreover, to de-stimulate the same NP having too many nodes in the same country,
            // the node rewards is reduced for each node the NP has in the given country. The reduction coefficient is
            // computed as the average of reduction coefficients on DC Country level.

            coeff = Decimal::from(rate.reward_coefficient_percent.unwrap_or(80)) / dec!(100);
            let coefficients = vec![coeff; *node_count as usize];
            let base_rewards = vec![base_rewards; *node_count as usize];
            let region_key = region_type3_key(region.to_string());

            type3_coefficients_rewards
                .entry(region_key)
                .and_modify(|(entry_coefficients, entry_rewards)| {
                    entry_coefficients.extend(&coefficients);
                    entry_rewards.extend(&base_rewards);
                })
                .or_insert((coefficients, base_rewards));
        } else {
            // For `rewardable_nodes` which are not type3* the base rewards for the sigle node is the entry
            // in the rewards table for the specific region (DC Continent + DC Country + DC State/City) and node type.

            region_nodetype_rewards.insert((region.clone(), node_type.clone()), base_rewards);
        }

        logger.add_entry(
            LogLevel::Mid,
            LogEntry::RewardTableEntry {
                node_type: node_type.to_string(),
                region: region.to_string(),
                coeff,
                base_rewards,
                node_count: *node_count,
            },
        );
    }

    // Computes node rewards for type3* nodes in all regions and add it to region_nodetype_rewards
    for (key, (coefficients, rewards)) in type3_coefficients_rewards {
        let rewards_len = rewards.len();
        let mut running_coefficient = dec!(1);
        let mut region_rewards = Vec::new();

        let coefficients_avg = logger.execute("Coefficients avg.", Operation::Avg(coefficients));
        let rewards_avg = logger.execute("Rewards avg.", Operation::Avg(rewards));
        for _ in 0..rewards_len {
            region_rewards.push(Operation::Multiply(rewards_avg, running_coefficient));
            running_coefficient *= coefficients_avg;
        }
        let region_rewards = logger.execute("Total rewards after coefficient reduction", Operation::SumOps(region_rewards));
        let region_rewards_avg = logger.execute(
            "Rewards average after coefficient reduction",
            Operation::Divide(region_rewards, Decimal::from(rewards_len)),
        );

        region_nodetype_rewards.insert(key, region_rewards_avg);
    }

    region_nodetype_rewards
}
