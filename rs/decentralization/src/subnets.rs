use std::collections::BTreeMap;

use ic_base_types::PrincipalId;
use ic_management_types::{
    requests::{NodeRemoval, NodeRemovalReason},
    MinNakamotoCoefficients, Status, Subnet,
};
use itertools::Itertools;
use log::{info, warn};

use crate::{
    network::{Node, SubnetChangeRequest},
    SubnetChangeResponse,
};

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
        nodes_with_proposals: std::collections::BTreeMap<ic_base_types::PrincipalId, ic_management_types::Node>,
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

pub enum ReplaceTarget {
    /// Subnet targeted for replacements
    Subnet(PrincipalId),
    /// Nodes on the same subnet that need to be replaced for other reasons
    Nodes { nodes: Vec<PrincipalId>, motivation: String },
}

pub struct MembershipReplace {
    pub target: ReplaceTarget,
    pub heal: bool,
    pub optimize: Option<usize>,
    pub exclude: Option<Vec<String>>,
    pub only: Vec<String>,
    pub include: Option<Vec<PrincipalId>>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
}

impl MembershipReplace {
    pub async fn replace(
        &self,
        subnet_health: BTreeMap<PrincipalId, Status>,
        registry_nodes: BTreeMap<PrincipalId, ic_management_types::Node>,
        subnet_change_request: SubnetChangeRequest,
    ) -> anyhow::Result<SubnetChangeResponse> {
        let mut motivations: Vec<String> = vec![];

        let mut replacements_unhealthy: Vec<Node> = Vec::new();
        if self.heal {
            let subnet = subnet_change_request.subnet();
            let unhealthy: Vec<Node> = subnet
                .nodes
                .into_iter()
                .filter_map(|n| match subnet_health.get(&n.id) {
                    Some(health) => {
                        if *health == ic_management_types::Status::Healthy {
                            None
                        } else {
                            info!("Node {} is {:?}", n.id, health);
                            Some(n)
                        }
                    }
                    None => {
                        warn!("Node {} has no known health, assuming unhealthy", n.id);
                        Some(n)
                    }
                })
                .collect::<Vec<_>>();

            if !unhealthy.is_empty() {
                // Do not check the health of the force-included nodes
                let unhealthy = unhealthy
                    .into_iter()
                    .filter(|n| !self.include.as_ref().unwrap_or(&vec![]).contains(&n.id))
                    .collect::<Vec<_>>();
                replacements_unhealthy.extend(unhealthy);
            }
        }
        let req_replace_nodes = if let ReplaceTarget::Nodes {
            nodes: req_replace_node_ids,
            motivation: _,
        } = &self.target
        {
            let req_replace_nodes = req_replace_node_ids
                .iter()
                .filter_map(|n| registry_nodes.get(n))
                .map(Node::from)
                .collect::<Vec<_>>();
            replacements_unhealthy.retain(|n| !req_replace_node_ids.contains(&n.id));
            req_replace_nodes
        } else {
            vec![]
        };

        let num_unhealthy = replacements_unhealthy.len();
        if !replacements_unhealthy.is_empty() {
            let replace_target = if num_unhealthy == 1 { "node" } else { "nodes" };
            motivations.push(format!("replacing {num_unhealthy} unhealthy {replace_target}"));
        }
        // Optimize the requested number of nodes, and remove unhealthy nodes if there
        // are any
        let replacements = replacements_unhealthy.into_iter().chain(req_replace_nodes).collect();
        let change = subnet_change_request.optimize(self.optimize.unwrap_or(0), &replacements)?;
        let num_optimized = change.removed().len() - replacements.len();
        if num_optimized > 0 {
            let replace_target = if num_optimized == 1 { "node" } else { "nodes" };
            motivations.push(format!("replacing {num_optimized} {replace_target} to improve subnet decentralization"));
        }

        Ok(SubnetChangeResponse::from(&change).with_motivation(motivations.join("; ")))
    }
}

// impl Display for MembershipReplaceRequest
impl std::fmt::Display for MembershipReplace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = match &self.target {
            ReplaceTarget::Subnet(subnet) => format!("subnet {}", subnet),
            ReplaceTarget::Nodes { nodes, motivation } => {
                format!("nodes {:?} ({})", nodes, motivation)
            }
        };
        write!(f, "target: {}", target)?;
        if self.heal {
            write!(f, " heal: {}", self.heal)?;
        }
        if let Some(optimize) = self.optimize {
            write!(f, " optimize: {}", optimize)?;
        }
        if let Some(exclude) = &self.exclude {
            if !exclude.is_empty() {
                write!(f, " exclude: {:?}", self.exclude)?;
            }
        }
        if !self.only.is_empty() {
            write!(f, " only: {:?}", self.only)?;
        }
        if let Some(include) = &self.include {
            if !include.is_empty() {
                write!(f, " include: {:?}", include)?;
            }
        }
        if let Some(min_nakamoto_coefficients) = &self.min_nakamoto_coefficients {
            write!(f, " min_nakamoto_coefficients: {:?}", min_nakamoto_coefficients)?;
        }
        Ok(())
    }
}
