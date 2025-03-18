use ic_base_types::{NodeId, PrincipalId};
use rust_decimal::Decimal;
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
    let mut nodes_by_provider: BTreeMap<PrincipalId, Vec<RewardableNode>> = BTreeMap::new();

    for node in rewardable_nodes {
        nodes_by_provider.entry(node.node_provider_id).or_default().push(node.clone());
    }
    nodes_by_provider
}

pub fn nodes_ids(rewardable_nodes: &[RewardableNode]) -> Vec<NodeId> {
    rewardable_nodes.iter().map(|node| node.node_id).collect()
}

pub fn myr_xdr(value: &Decimal) -> String {
    format!("{} myrXDR", value.round())
}

pub fn round(value: &Decimal) -> String {
    value.round_dp(4).to_string()
}

pub fn avg(values: &[Decimal]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(values.len().max(1))
}
