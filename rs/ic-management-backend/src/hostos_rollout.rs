use anyhow::anyhow;
use futures_util::future::try_join;
use ic_base_types::{NodeId, PrincipalId};
use ic_management_types::requests::{HostosRolloutReason, HostosRolloutResponse, HostosRolloutSubnetAffected};
use ic_management_types::{
    Network, Node, NodeAssignment, NodeGroup, NodeGroupUpdate, NodeOwner, Status, Subnet,
    UpdateNodesHostosVersionsProposal,
};
use log::{debug, info};
use proposal::ProposalAgent;
use std::collections::BTreeMap;

use crate::health;
use crate::proposal;

#[derive(Clone)]
pub struct HostosRollout {
    pub grouped_nodes: BTreeMap<NodeGroup, Vec<Node>>,
    pub subnets: BTreeMap<PrincipalId, Subnet>,
    pub network: Network,
    pub proposal_agent: ProposalAgent,
    pub version: String,
}
impl HostosRollout {
    pub fn new(
        nodes: BTreeMap<PrincipalId, Node>,
        subnets: BTreeMap<PrincipalId, Subnet>,
        network: Network,
        proposal_agent: ProposalAgent,
        rollout_version: &str,
    ) -> Self {
        let grouped_nodes: BTreeMap<NodeGroup, Vec<Node>> = nodes
            .values()
            .cloned()
            .map(|node| {
                let (assignment, owner) = (
                    if node.subnet_id.is_some() {
                        NodeAssignment::Assigned
                    } else {
                        NodeAssignment::Unassigned
                    },
                    if let Some(dfinity_owned) = node.dfinity_owned {
                        if dfinity_owned {
                            NodeOwner::Dfinity
                        } else {
                            NodeOwner::Others
                        }
                    } else {
                        NodeOwner::Others
                    },
                );
                (NodeGroup::new(assignment, owner), node)
            })
            .fold(BTreeMap::new(), |mut acc, (node_group, node)| {
                acc.entry(node_group).or_insert_with(Vec::new).push(node);
                acc
            });

        HostosRollout {
            grouped_nodes,
            subnets,
            network,
            proposal_agent,
            version: rollout_version.to_string(),
        }
    }
    async fn nodes_different_version(&self, nodes: Vec<Node>) -> Option<Vec<Node>> {
        let nodes_different_version = nodes
            .iter()
            .filter(|n| n.hostos_version != self.version)
            .cloned()
            .collect::<Vec<_>>();

        if !nodes_different_version.is_empty() {
            return Some(nodes_different_version);
        }
        None
    }
    async fn nodes_without_proposals(
        &self,
        nodes: Vec<Node>,
        nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>,
    ) -> Option<Vec<Node>> {
        let nodes_with_open_proposals_ids = nodes_with_open_proposals
            .into_iter()
            .flat_map(|proposal| proposal.node_ids)
            .collect::<Vec<_>>();

        let nodes_filtered: Vec<Node> = nodes
            .iter()
            .cloned()
            .filter_map(|n| {
                let node_id: NodeId = NodeId::from(n.principal);
                if nodes_with_open_proposals_ids.contains(&node_id) {
                    debug!(
                        "Skipping update on node: {} because there is an open proposal for it",
                        &node_id
                    );
                    None
                } else {
                    Some(n)
                }
            })
            .collect();

        if !nodes_filtered.is_empty() {
            return Some(nodes_filtered);
        }
        None
    }

    async fn nodes_by_status(
        &self,
        nodes: Vec<Node>,
        nodes_health: BTreeMap<PrincipalId, Status>,
    ) -> BTreeMap<Status, Vec<Node>> {
        let nodes_by_status = nodes
            .iter()
            .cloned()
            .map(|node| {
                (
                    nodes_health.get(&node.principal).cloned().unwrap_or(Status::Unknown),
                    node,
                )
            })
            .fold(BTreeMap::new(), |mut acc, (status, node)| {
                acc.entry(status).or_insert_with(Vec::new).push(node);
                acc
            });

        nodes_by_status
            .iter()
            .for_each(|(status, n)| info!("STATUS {}: found {} nodes", status, n.len()));
        nodes_by_status
    }

    async fn take_from_subnets(&self, candidate_nodes: Vec<Node>, update_group: NodeGroupUpdate) -> Vec<Node> {
        self.subnets
            .iter()
            .flat_map(|(subnet_id, subnet)| {
                let subnet_size = subnet.nodes.len();
                let nodes_to_take = update_group.nodes_to_take(subnet_size);
                let nodes = subnet
                    .nodes
                    .iter()
                    .cloned()
                    .filter(|n| candidate_nodes.iter().any(|node| node.principal == n.principal))
                    .take(nodes_to_take)
                    .collect::<Vec<_>>();
                let actual_percent = nodes.len() as f32 / subnet_size as f32 * 100.0;

                if nodes.is_empty() {
                    info!("All valid nodes in the subnet: {} have been updated", subnet_id);
                    None
                } else {
                    if nodes.len() < nodes_to_take {
                        debug!(
                            "Updating all valid nodes ({}) left in the subnet: {}\n\
                                {}% of all nodes in the subnet",
                            nodes.len(),
                            subnet_id,
                            actual_percent
                        );
                    } else {
                        debug!(
                            "Updating {} valid nodes in the subnet: {}\n\
                                {}% of all nodes in the subnet",
                            nodes.len(),
                            subnet_id,
                            actual_percent
                        );
                    }
                    Some(nodes)
                }
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    async fn with_nodes_health_and_open_proposals(
        &self,
        nodes_health: BTreeMap<PrincipalId, Status>,
        nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>,
        update_group: NodeGroupUpdate,
    ) -> anyhow::Result<HostosRolloutResponse> {
        info!("CANDIDATES SELECTION FOR {:?}", &update_group);

        info!("Selecting candidate nodes in the group");
        let nodes_in_group = self
            .grouped_nodes
            .get(&update_group.node_group)
            .ok_or(anyhow!("No candidates selected for: {}", update_group.node_group))?
            .clone();

        info!("Fetching candidate nodes health status");
        let nodes_by_status = self.nodes_by_status(nodes_in_group.clone(), nodes_health).await;

        info!("Selecting healthy candidate nodes");
        let nodes_healthy = match nodes_by_status.get(&Status::Healthy) {
            Some(nodes_by_status) => nodes_by_status.clone(),
            None => {
                return Ok(HostosRolloutResponse::None(HostosRolloutReason::NoNodeHealthy));
            }
        };

        info!("Filtering out candidate nodes with an open proposal");
        let nodes_without_proposals = match self
            .nodes_without_proposals(nodes_healthy, nodes_with_open_proposals)
            .await
        {
            Some(nodes_without_proposals) => nodes_without_proposals,
            None => {
                return Ok(HostosRolloutResponse::None(HostosRolloutReason::NoNodeWithoutProposal));
            }
        };

        info!("Filtering out candidate nodes already updated");
        let candidate_nodes = match self.nodes_different_version(nodes_without_proposals).await {
            Some(nodes_different_version) => nodes_different_version,
            None => {
                return Ok(HostosRolloutResponse::None(HostosRolloutReason::AllAlreadyUpdated));
            }
        };

        info!("Selecting percent nodes from candidate nodes");
        let response = match update_group.node_group.assignment {
            NodeAssignment::Unassigned => {
                let nodes_to_take = update_group.nodes_to_take(nodes_in_group.len());
                let nodes_to_update = candidate_nodes.into_iter().take(nodes_to_take).collect::<Vec<_>>();
                HostosRolloutResponse::Ok(nodes_to_update, None)
            }
            NodeAssignment::Assigned => {
                let nodes_to_update = self.take_from_subnets(candidate_nodes, update_group).await;
                let subnets_affected = self
                    .subnets
                    .values()
                    .cloned()
                    .filter_map(|subnet| {
                        if nodes_to_update
                            .iter()
                            .any(|node| node.subnet_id.unwrap_or_default() == subnet.principal)
                        {
                            Some(HostosRolloutSubnetAffected {
                                subnet_id: subnet.principal,
                                subnet_size: subnet.nodes.len(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<HostosRolloutSubnetAffected>>();

                HostosRolloutResponse::Ok(nodes_to_update, Some(subnets_affected))
            }
        };

        Ok(response)
    }

    pub async fn execute(&self, update_group: NodeGroupUpdate) -> anyhow::Result<HostosRolloutResponse> {
        let (nodes_health, nodes_with_open_proposals) = try_join(
            health::HealthClient::new(self.network.clone()).nodes(),
            self.proposal_agent.list_open_update_nodes_hostos_versions_proposals(),
        )
        .await?;

        self.with_nodes_health_and_open_proposals(nodes_health, nodes_with_open_proposals, update_group)
            .await
    }
}

#[cfg(test)]
pub mod test {
    use crate::hostos_rollout::NodeAssignment::{Assigned, Unassigned};
    use crate::hostos_rollout::NodeOwner::{Dfinity, Others};
    use ic_management_types::{Network, Node, NumberOfNodes, Operator, Provider, Subnet};
    use std::net::Ipv6Addr;

    use super::*;

    #[tokio::test]
    async fn test_hostos_rollout() {
        let version_one = "ec140b74dc4fef2f4bee3fad936e315380fa5af3".to_string();
        let version_two = "e268b9807f1ab4ae65d7b29fe70a3b358d014d6a".to_string();

        let subnet_id = PrincipalId::new_subnet_test_id(0);
        let assigned_dfinity = gen_test_nodes(Some(subnet_id), 10, 0, version_one.clone(), true);
        let unassigned_dfinity_nodes = gen_test_nodes(None, 10, 10, version_one.clone(), true);
        let assigned_others_nodes = gen_test_nodes(Some(subnet_id), 10, 20, version_two.clone(), false);
        let union: BTreeMap<PrincipalId, Node> = {
            assigned_dfinity
                .clone()
                .into_iter()
                .chain(unassigned_dfinity_nodes.clone())
                .chain(assigned_others_nodes.clone())
                .collect()
        };
        let union_nodes = union.values().cloned().collect::<Vec<_>>();

        let subnet = {
            let mut sub = BTreeMap::new();
            sub.insert(
                subnet_id,
                Subnet {
                    principal: subnet_id,
                    nodes: union_nodes.clone(),
                    ..Default::default()
                },
            );
            sub
        };

        let healthy_nodes = union
            .keys()
            .cloned()
            .map(|principal| (principal, Status::Healthy))
            .collect::<BTreeMap<PrincipalId, Status>>();

        let open_proposals: Vec<UpdateNodesHostosVersionsProposal> = vec![];

        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            Network::Mainnet,
            ProposalAgent::new("https://ic0.app".to_string()),
            version_one.clone().as_str(),
        );

        let results = hostos_rollout
            .clone()
            .with_nodes_health_and_open_proposals(
                healthy_nodes.clone(),
                open_proposals.clone(),
                NodeGroupUpdate::new_all(Assigned, Others),
            )
            .await
            .unwrap()
            .unwrap()
            .iter()
            .map(|n| n.principal)
            .collect::<Vec<_>>();
        let want = assigned_others_nodes.values().map(|n| n.principal).collect::<Vec<_>>();

        assert_eq!(results, want, "assigned_others_nodes should be updated");

        let want = 10usize;

        assert_eq!(results.len(), want, "2 nodes should be updated");

        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            Network::Mainnet,
            ProposalAgent::new("https://ic0.app".to_string()),
            version_two.clone().as_str(),
        );

        let results = hostos_rollout
            .with_nodes_health_and_open_proposals(
                healthy_nodes.clone(),
                open_proposals.clone(),
                NodeGroupUpdate::new(Unassigned, Dfinity, NumberOfNodes::Percentage(10)),
            )
            .await
            .unwrap()
            .unwrap()
            .iter()
            .map(|n| n.principal)
            .collect::<Vec<_>>()[0];

        let want = unassigned_dfinity_nodes
            .clone()
            .iter()
            .next()
            .map(|(_, n)| n.principal)
            .unwrap();

        assert_eq!(results, want, "the first unassigned_dfinity_node should be updated");
    }

    fn gen_test_nodes(
        subnet_id: Option<PrincipalId>,
        num_nodes: u64,
        start_at_number: u64,
        hostos_version: String,
        dfinity_owned: bool,
    ) -> BTreeMap<PrincipalId, Node> {
        let mut n = BTreeMap::new();
        for i in start_at_number..start_at_number + num_nodes {
            let node = Node {
                principal: PrincipalId::new_node_test_id(i),
                ip_addr: Ipv6Addr::LOCALHOST,
                operator: Operator {
                    principal: PrincipalId::new_node_test_id(i),
                    provider: Provider {
                        principal: PrincipalId::new_node_test_id(i),
                        name: None,
                        website: None,
                    },
                    allowance: 23933,
                    datacenter: None,
                },
                hostname: None,
                hostos_release: None,
                proposal: None,
                label: None,
                decentralized: false,
                duplicates: None,
                subnet_id,
                hostos_version: hostos_version.clone(),
                dfinity_owned: Some(dfinity_owned),
            };
            n.insert(node.principal, node);
        }
        n
    }
}
