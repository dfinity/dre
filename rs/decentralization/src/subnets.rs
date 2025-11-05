use ic_base_types::PrincipalId;
use ic_management_types::{
    HealthStatus, Subnet,
    requests::{NodeRemoval, NodeRemovalReason},
};
use indexmap::IndexMap;
use itertools::Itertools;
use std::sync::Arc;

use crate::network::DecentralizedSubnet;

pub fn unhealthy_with_nodes(
    subnets: &IndexMap<PrincipalId, Subnet>,
    nodes_health: &IndexMap<PrincipalId, HealthStatus>,
) -> IndexMap<PrincipalId, Vec<ic_management_types::Node>> {
    subnets
        .clone()
        .into_iter()
        .filter_map(|(subnet_id, subnet)| {
            let unhealthy = subnet
                .nodes
                .into_iter()
                .filter_map(|n| match nodes_health.get(&n.principal) {
                    Some(health) if *health == ic_management_types::HealthStatus::Healthy => None,
                    _ => Some(n),
                })
                .collect::<Vec<_>>();

            if !unhealthy.is_empty() { Some((subnet_id, unhealthy)) } else { None }
        })
        .collect::<IndexMap<_, _>>()
}

pub fn subnets_with_business_rules_violations(subnets: &[Subnet]) -> Vec<Subnet> {
    subnets
        .iter()
        .filter_map(|subnet| {
            let decentralized_subnet = DecentralizedSubnet::from(subnet.clone());

            if decentralized_subnet
                .check_business_rules()
                .expect("business rules check should succeed")
                .0
                > 0
            {
                Some(subnet.clone())
            } else {
                None
            }
        })
        .collect_vec()
}

pub struct NodesRemover {
    pub no_auto: bool,
    pub remove_degraded: bool,
    pub extra_nodes_filter: Vec<String>,
    pub exclude: Option<Vec<String>>,
    pub motivation: String,
}
impl NodesRemover {
    pub fn remove_nodes(
        &self,
        mut healths: IndexMap<ic_base_types::PrincipalId, ic_management_types::HealthStatus>,
        nodes_with_proposals: Arc<IndexMap<ic_base_types::PrincipalId, ic_management_types::Node>>,
    ) -> (Vec<NodeRemoval>, String) {
        let nodes_to_rm = nodes_with_proposals
            .values()
            .cloned()
            .map(|n| {
                let status = healths.shift_remove(&n.principal).unwrap_or(ic_management_types::HealthStatus::Unknown);
                (n, status)
            })
            .filter(|(n, _)| n.proposal.is_none())
            .filter_map(|(n, status)| {
                if n.subnet_id.is_some() {
                    return None;
                }

                if let Some(exclude) = self.exclude.as_ref() {
                    for exclude_feature in exclude {
                        if n.matches_feature_value(exclude_feature) {
                            return None;
                        }
                    }
                }

                if let Some(filter) = self.extra_nodes_filter.iter().find(|f| n.matches_feature_value(f)) {
                    return Some(NodeRemoval {
                        node: n,
                        reason: NodeRemovalReason::MatchedFilter(filter.clone()),
                    });
                }

                if !self.no_auto {
                    if let Some(principal) = n.duplicates {
                        return Some(NodeRemoval {
                            node: n,
                            reason: NodeRemovalReason::Duplicates(principal),
                        });
                    }
                    let should_remove_node = if self.remove_degraded {
                        matches!(status, ic_management_types::HealthStatus::Dead) || matches!(status, ic_management_types::HealthStatus::Degraded)
                    } else {
                        matches!(status, ic_management_types::HealthStatus::Dead)
                    };
                    if should_remove_node {
                        return Some(NodeRemoval {
                            node: n,
                            reason: NodeRemovalReason::Unhealthy(status),
                        });
                    }
                }

                None
            })
            .collect::<Vec<_>>();

        let motivation = "\n".to_string()
            + &nodes_to_rm
                .iter()
                .map(|nr| match nr.reason {
                    ic_management_types::requests::NodeRemovalReason::Duplicates(_)
                    | ic_management_types::requests::NodeRemovalReason::Unhealthy(_) => "Removing unhealthy nodes from the network, for redeployment",
                    ic_management_types::requests::NodeRemovalReason::MatchedFilter(_) => self.motivation.as_str(),
                })
                .unique()
                .map(|m| format!(" * {m}"))
                .collect::<Vec<_>>()
                .join("\n");

        (nodes_to_rm, motivation)
    }
}
