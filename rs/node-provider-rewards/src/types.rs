use ic_base_types::{NodeId, PrincipalId};
use itertools::Itertools;
use std::collections::BTreeMap;

#[derive(Eq, Hash, PartialEq, Clone, Ord, PartialOrd)]
pub struct NodeCategory {
    pub region: String,
    pub node_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RewardableNode {
    pub node_id: NodeId,
    pub node_provider_id: PrincipalId,
    pub region: String,
    pub node_type: String,
}

impl RewardableNode {
    pub fn category(&self) -> NodeCategory {
        NodeCategory {
            region: self.region.clone(),
            node_type: self.node_type.clone(),
        }
    }
}

pub fn rewardable_nodes_by_provider(rewardable_nodes: &[RewardableNode]) -> BTreeMap<PrincipalId, Vec<RewardableNode>> {
    rewardable_nodes
        .iter()
        .cloned()
        .into_group_map_by(|node| node.node_provider_id)
        .into_iter()
        .collect()
}
