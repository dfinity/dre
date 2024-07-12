use std::{collections::BTreeMap, sync::Arc};

use crate::network::Node;
use ic_base_types::PrincipalId;
use ic_management_types::{
    requests::{NodeRemoval, NodeRemovalReason},
    Status, Subnet,
};
use itertools::Itertools;

pub async fn unhealthy_with_nodes(
    subnets: &BTreeMap<PrincipalId, Subnet>,
    nodes_health: BTreeMap<PrincipalId, Status>,
) -> BTreeMap<PrincipalId, Vec<ic_management_types::Node>> {
    subnets
        .clone()
        .into_iter()
        .filter_map(|(id, subnet)| {
            let unhealthy = subnet
                .nodes
                .into_iter()
                .filter_map(|n| match nodes_health.get(&n.principal) {
                    Some(health) if *health == ic_management_types::Status::Healthy => None,
                    _ => Some(n),
                })
                .collect::<Vec<_>>();

            if !unhealthy.is_empty() {
                Some((id, unhealthy))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>()
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
        mut healths: std::collections::BTreeMap<ic_base_types::PrincipalId, ic_management_types::Status>,
        nodes_with_proposals: Arc<std::collections::BTreeMap<ic_base_types::PrincipalId, ic_management_types::Node>>,
    ) -> (Vec<NodeRemoval>, String) {
        let nodes_to_rm = nodes_with_proposals
            .values()
            .cloned()
            .map(|n| {
                let status = healths.remove(&n.principal).unwrap_or(ic_management_types::Status::Unknown);
                (n, status)
            })
            .filter(|(n, _)| n.proposal.is_none())
            .filter_map(|(n, status)| {
                if n.subnet_id.is_some() {
                    return None;
                }

                let decentralization_node = Node::from(&n);

                if let Some(exclude) = self.exclude.as_ref() {
                    for exclude_feature in exclude {
                        if decentralization_node.matches_feature_value(exclude_feature) {
                            return None;
                        }
                    }
                }

                if let Some(filter) = self.extra_nodes_filter.iter().find(|f| decentralization_node.matches_feature_value(f)) {
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
                        matches!(status, ic_management_types::Status::Dead) || matches!(status, ic_management_types::Status::Degraded)
                    } else {
                        matches!(status, ic_management_types::Status::Dead)
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
