use super::*;
use crate::nakamoto::NakamotoScore;

#[derive(Debug, Clone, Default)]
pub struct SubnetChange {
    pub subnet_id: PrincipalId,
    pub old_nodes: Vec<Node>,
    pub new_nodes: Vec<Node>,
    pub removed_nodes: Vec<Node>,
    pub added_nodes: Vec<Node>,
    pub penalties_before_change: (usize, Vec<String>),
    pub penalties_after_change: (usize, Vec<String>),
    pub comment: Option<String>,
    pub run_log: Vec<String>,
}

impl SubnetChange {
    pub fn with_nodes(self, nodes_to_add: &[Node]) -> Self {
        let new_nodes = [self.new_nodes, nodes_to_add.to_vec()].concat();
        let penalties_before_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet_id, &self.old_nodes)
            .expect("Business rules check before should succeed");
        let penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet_id, &new_nodes)
            .expect("Business rules check after should succeed");
        Self {
            new_nodes,
            added_nodes: nodes_to_add.to_vec(),
            penalties_before_change,
            penalties_after_change,
            ..self
        }
    }

    pub fn without_nodes(mut self, nodes_to_remove: &[Node]) -> Self {
        let nodes_to_rm = AHashSet::from_iter(nodes_to_remove);
        self.removed_nodes.extend(nodes_to_remove.to_vec());
        self.new_nodes.retain(|n| !nodes_to_rm.contains(n));
        self.penalties_before_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet_id, &self.old_nodes)
            .expect("Business rules check before should succeed");
        self.penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet_id, &self.new_nodes)
            .expect("Business rules check after should succeed");
        self
    }

    pub fn added(&self) -> Vec<Node> {
        self.added_nodes.clone()
    }

    pub fn removed(&self) -> Vec<Node> {
        self.removed_nodes.clone()
    }

    pub fn before(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.subnet_id,
            nodes: self.old_nodes.clone(),
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            comment: self.comment.clone(),
            run_log: Vec::new(),
        }
    }

    pub fn after(&self) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: self.subnet_id,
            nodes: self.new_nodes.clone(),
            added_nodes: self.added_nodes.clone(),
            removed_nodes: self.removed_nodes.clone(),
            comment: self.comment.clone(),
            run_log: self.run_log.clone(),
        }
    }
}

pub fn generate_removed_nodes_description(subnet_nodes: &[Node], remove_nodes: &[Node]) -> Vec<(Node, String)> {
    let mut subnet_nodes: AHashMap<PrincipalId, Node> = AHashMap::from_iter(subnet_nodes.iter().map(|n| (n.principal, n.clone())));
    let mut result = Vec::new();
    for node in remove_nodes {
        let nakamoto_before = NakamotoScore::new_from_nodes(subnet_nodes.values());
        subnet_nodes.remove(&node.principal);
        let nakamoto_after = NakamotoScore::new_from_nodes(subnet_nodes.values());
        let nakamoto_diff = nakamoto_after.describe_difference_from(&nakamoto_before).1;

        result.push((node.clone(), nakamoto_diff));
    }
    result
}

pub fn generate_added_node_description(subnet_nodes: &[Node], add_nodes: &[Node]) -> Vec<(Node, String)> {
    let mut subnet_nodes: AHashMap<PrincipalId, Node> = AHashMap::from_iter(subnet_nodes.iter().map(|n| (n.principal, n.clone())));
    let mut result = Vec::new();
    for node in add_nodes {
        let nakamoto_before = NakamotoScore::new_from_nodes(subnet_nodes.values());
        subnet_nodes.insert(node.principal, node.clone());
        let nakamoto_after = NakamotoScore::new_from_nodes(subnet_nodes.values());
        let nakamoto_diff = nakamoto_after.describe_difference_from(&nakamoto_before).1;

        result.push((node.clone(), nakamoto_diff));
    }
    result
}
