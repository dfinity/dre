use ic_base_types::{NodeId, PrincipalId};
use std::collections::BTreeMap;

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct RegionNodeTypeCategory {
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
    pub fn region_node_type_category(&self) -> RegionNodeTypeCategory {
        RegionNodeTypeCategory {
            region: self.region.clone(),
            node_type: self.node_type.clone(),
        }
    }
}

pub fn rewardable_nodes_by_provider(rewardable_nodes: &[RewardableNode]) -> BTreeMap<PrincipalId, Vec<RewardableNode>> {
    let mut nodes_by_provider: BTreeMap<PrincipalId, Vec<RewardableNode>> = BTreeMap::new();

    for node in rewardable_nodes {
        nodes_by_provider.entry(node.node_provider_id).or_default().push(node.clone());
    }
    nodes_by_provider
}

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}
