use super::*;
use log::info;

#[derive(Default, Clone, Debug)]
pub struct SubnetChangeRequest {
    pub(crate) subnet: DecentralizedSubnet,
    pub(crate) available_nodes: Vec<Node>,
    pub(crate) include_nodes: Vec<Node>,
    pub(crate) nodes_to_remove: Vec<Node>,
    pub(crate) nodes_to_keep: Vec<Node>,
}

impl SubnetChangeRequest {
    pub fn new(
        subnet: DecentralizedSubnet,
        available_nodes: Vec<Node>,
        include_nodes: Vec<Node>,
        nodes_to_remove: Vec<Node>,
        nodes_to_keep: Vec<Node>,
    ) -> Self {
        SubnetChangeRequest {
            subnet,
            available_nodes,
            include_nodes,
            nodes_to_remove,
            nodes_to_keep,
        }
    }

    pub fn keeping_from_used<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        let mut change_new = self.clone();
        let nodes_to_keep = self
            .subnet
            .nodes
            .into_iter()
            .filter(|node: &Node| nodes.iter().match_any(node))
            .collect_vec();
        change_new.nodes_to_keep.extend(nodes_to_keep);
        change_new
    }

    pub fn removing_from_used<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        let mut change_new = self.clone();
        let nodes_to_remove = self
            .subnet
            .nodes
            .into_iter()
            .filter(|node: &Node| nodes.iter().match_any(node))
            .collect_vec();
        change_new.nodes_to_remove.extend(nodes_to_remove);
        change_new
    }

    pub fn including_from_available<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        Self {
            include_nodes: self
                .available_nodes
                .iter()
                .filter(|node| nodes.iter().match_any(node))
                .cloned()
                .collect_vec(),
            ..self
        }
    }

    pub fn excluding_from_available<T: Identifies<Node>>(self, nodes: Vec<T>) -> Self {
        Self {
            available_nodes: self
                .available_nodes
                .iter()
                .filter(|node| !nodes.iter().match_any(node))
                .cloned()
                .collect_vec(),
            ..self
        }
    }

    pub fn subnet(&self) -> DecentralizedSubnet {
        self.subnet.clone()
    }

    pub fn with_custom_available_nodes(self, nodes: Vec<Node>) -> Self {
        Self {
            available_nodes: nodes,
            ..self
        }
    }

    pub fn optimize(
        mut self,
        optimize_count: usize,
        replacements_unhealthy: &[Node],
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &[Node],
    ) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();
        self.subnet = self.subnet.without_nodes(replacements_unhealthy)?;
        let result = self.resize(
            optimize_count + replacements_unhealthy.len(),
            optimize_count,
            replacements_unhealthy.len(),
            health_of_nodes,
            cordoned_features,
            all_nodes,
        )?;
        Ok(SubnetChange { old_nodes, ..result })
    }

    pub fn rescue(
        mut self,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &[Node],
    ) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();
        let nodes_to_remove = self
            .subnet
            .nodes
            .iter()
            .filter(|n| !self.nodes_to_keep.contains(n))
            .cloned()
            .collect_vec();
        self.subnet = self.subnet.without_nodes(&nodes_to_remove)?;

        info!("Nodes left in the subnet:\n{:#?}", &self.subnet.nodes);
        let result = self.resize(
            self.subnet.removed_nodes.len(),
            0,
            self.subnet.removed_nodes.len(),
            health_of_nodes,
            cordoned_features,
            all_nodes,
        )?;
        Ok(SubnetChange { old_nodes, ..result })
    }

    /// Add or remove nodes from the subnet.
    pub fn resize(
        &self,
        how_many_nodes_to_add: usize,
        how_many_nodes_to_remove: usize,
        how_many_nodes_unhealthy: usize,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &[Node],
    ) -> Result<SubnetChange, NetworkError> {
        let old_nodes = self.subnet.nodes.clone();

        let all_healthy_nodes = self
            .available_nodes
            .clone()
            .into_iter()
            .filter(|n| !self.include_nodes.contains(n))
            .filter(|n| health_of_nodes.get(&n.principal).unwrap_or(&HealthStatus::Unknown) == &HealthStatus::Healthy)
            .collect::<Vec<_>>();

        let available_nodes = all_healthy_nodes
            .into_iter()
            .filter(|n| {
                for cordoned_feature in &cordoned_features {
                    if let Some(node_feature) = n.get_feature(&cordoned_feature.feature) {
                        if PartialEq::eq(&node_feature, &cordoned_feature.value) {
                            // Node contains cordoned feature
                            // exclude it from available pool
                            return false;
                        }
                    }
                }
                // Node doesn't contain any cordoned features
                // include it the available pool
                true
            })
            .collect_vec();

        info!(
            "Evaluating change in subnet {} membership: removing ({} healthy + {} unhealthy) and adding {} node. Total available {} healthy nodes.",
            self.subnet.id.to_string().split_once('-').expect("subnet id is expected to have a -").0,
            how_many_nodes_to_remove + self.nodes_to_remove.len(),
            how_many_nodes_unhealthy,
            how_many_nodes_to_add + self.include_nodes.len(),
            available_nodes.len(),
        );

        let resized_subnet = if how_many_nodes_to_remove > 0 {
            self.subnet
                .clone()
                .subnet_with_fewer_nodes(how_many_nodes_to_remove, all_nodes)
                .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
        } else {
            self.subnet.clone()
        };

        let available_nodes = available_nodes
            .iter()
            .cloned()
            .chain(resized_subnet.removed_nodes.clone())
            .filter(|n| health_of_nodes.get(&n.principal).unwrap_or(&HealthStatus::Unknown) == &HealthStatus::Healthy)
            .filter(|n| {
                for cordoned_feature in &cordoned_features {
                    if let Some(node_feature) = n.get_feature(&cordoned_feature.feature) {
                        if PartialEq::eq(&node_feature, &cordoned_feature.value) {
                            // Node contains cordoned feature
                            // exclude it from available pool
                            return false;
                        }
                    }
                }
                // Node doesn't contain any cordoned features
                // include it the available pool
                true
            })
            .collect::<Vec<_>>();
        let resized_subnet = resized_subnet
            .with_nodes(&self.include_nodes)
            .without_nodes(&self.nodes_to_remove)?
            .subnet_with_more_nodes(how_many_nodes_to_add, &available_nodes, all_nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?
            .without_duplicate_added_removed();

        let penalties_before_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet.id, &old_nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?;

        let penalties_after_change = DecentralizedSubnet::check_business_rules_for_subnet_with_nodes(&self.subnet.id, &resized_subnet.nodes)
            .map_err(|e| NetworkError::ResizeFailed(e.to_string()))?;

        let subnet_change = SubnetChange {
            subnet_id: self.subnet.id,
            old_nodes,
            new_nodes: resized_subnet.nodes,
            removed_nodes: resized_subnet.removed_nodes,
            added_nodes: resized_subnet.added_nodes,
            penalties_before_change,
            penalties_after_change,
            comment: resized_subnet.comment,
            run_log: resized_subnet.run_log,
        };
        Ok(subnet_change)
    }

    /// Evaluates the subnet change request to simulate the requested topology
    /// change. Command returns all the information about the subnet before
    /// and after the change.
    pub fn evaluate(
        self,
        health_of_nodes: &IndexMap<PrincipalId, HealthStatus>,
        cordoned_features: Vec<CordonedFeature>,
        all_nodes: &[Node],
    ) -> Result<SubnetChange, NetworkError> {
        self.resize(0, 0, 0, health_of_nodes, cordoned_features, all_nodes)
    }
}
