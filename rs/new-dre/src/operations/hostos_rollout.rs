use anyhow::anyhow;
use async_recursion::async_recursion;
use futures_util::future::try_join;
use ic_base_types::{NodeId, PrincipalId};
use ic_management_backend::health::{self, HealthStatusQuerier};
use ic_management_backend::proposal::ProposalAgent;
use ic_management_types::{Network, Node, Status, Subnet, UpdateNodesHostosVersionsProposal};
use log::{debug, info, warn};
use std::sync::Arc;
use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use crate::commands::hostos::rollout_from_node_group::{NodeAssignment, NodeOwner};

pub enum HostosRolloutResponse {
    Ok(Vec<Node>, Option<Vec<HostosRolloutSubnetAffected>>),
    None(Vec<(NodeGroup, HostosRolloutReason)>),
}

#[derive(Clone, Eq, PartialEq)]
pub struct HostosRolloutSubnetAffected {
    pub subnet_id: PrincipalId,
    pub subnet_size: usize,
}

#[derive(Eq, PartialEq, Debug)]
pub enum HostosRolloutReason {
    NoNodeHealthy,
    NoNodeWithoutProposal,
    AllAlreadyUpdated,
}

impl Display for HostosRolloutReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoNodeHealthy => write!(f, "No healthy node found in the group"),
            Self::NoNodeWithoutProposal => write!(f, "No node without open proposals found in the group"),
            Self::AllAlreadyUpdated => write!(f, "All candidate nodes have been already updated"),
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct NodeGroup {
    pub assignment: NodeAssignment,
    pub owner: NodeOwner,
}

impl NodeGroup {
    pub fn new(assignment: NodeAssignment, owner: NodeOwner) -> Self {
        NodeGroup { assignment, owner }
    }
}
impl std::fmt::Display for NodeGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GROUP {{ subnet: {:?}, owner: {:?} }}", self.assignment, self.owner)
    }
}
#[derive(Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub enum NumberOfNodes {
    Percentage(i32),
    Absolute(i32),
}
impl FromStr for NumberOfNodes {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.ends_with('%') {
            let percentage = i32::from_str(input.trim_end_matches('%'))?;
            if (0..=100).contains(&percentage) {
                Ok(NumberOfNodes::Percentage(percentage))
            } else {
                Err(anyhow!("Percentage must be between 0 and 100"))
            }
        } else {
            Ok(NumberOfNodes::Absolute(i32::from_str(input)?))
        }
    }
}

impl Default for NumberOfNodes {
    fn default() -> Self {
        NumberOfNodes::Percentage(100)
    }
}

impl std::fmt::Display for NumberOfNodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberOfNodes::Percentage(x) => write!(f, "{}% of", x),
            NumberOfNodes::Absolute(x) => write!(f, "{}", x),
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct NodeGroupUpdate {
    pub node_group: NodeGroup,
    pub maybe_number_nodes: Option<NumberOfNodes>,
}
impl NodeGroupUpdate {
    pub fn new(assignment: Option<NodeAssignment>, owner: Option<NodeOwner>, nodes_per_subnet: NumberOfNodes) -> Self {
        NodeGroupUpdate {
            node_group: NodeGroup::new(assignment.unwrap_or_default(), owner.unwrap_or_default()),
            maybe_number_nodes: Some(nodes_per_subnet),
        }
    }

    pub fn with_assignment(&self, assignment: NodeAssignment) -> Self {
        Self {
            node_group: NodeGroup {
                assignment,
                owner: self.node_group.owner,
            },
            maybe_number_nodes: self.maybe_number_nodes,
        }
    }
    pub fn nodes_to_take(&self, group_size: usize) -> usize {
        match self.maybe_number_nodes.unwrap_or_default() {
            NumberOfNodes::Percentage(percent_to_update) => (group_size as f32 * percent_to_update as f32 / 100.0).floor() as usize,
            NumberOfNodes::Absolute(number_nodes) => number_nodes as usize,
        }
    }
}

enum CandidatesSelection {
    Ok(Vec<Node>),
    None(HostosRolloutReason),
}

#[derive(Clone)]
pub struct HostosRollout {
    nodes_all: Vec<Node>,
    pub grouped_nodes: BTreeMap<NodeGroup, Vec<Node>>,
    pub subnets: Arc<BTreeMap<PrincipalId, Subnet>>,
    pub network: Network,
    pub proposal_agent: ProposalAgent,
    pub only_filter: Vec<String>,
    pub exclude_filter: Vec<String>,
    pub version: String,
}
impl HostosRollout {
    pub fn new(
        nodes: Arc<BTreeMap<PrincipalId, Node>>,
        subnets: Arc<BTreeMap<PrincipalId, Subnet>>,
        network: &Network,
        proposal_agent: ProposalAgent,
        rollout_version: &str,
        only_filter: &[String],
        exclude_filter: &[String],
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
                acc.entry(node_group).or_default().push(node);
                acc
            });

        HostosRollout {
            nodes_all: nodes.values().cloned().collect(),
            grouped_nodes,
            subnets,
            network: network.clone(),
            proposal_agent,
            only_filter: only_filter.to_vec(),
            exclude_filter: exclude_filter.to_vec(),
            version: rollout_version.to_string(),
        }
    }

    async fn nodes_different_version(&self, nodes: Vec<Node>) -> Vec<Node> {
        nodes.into_iter().filter(|n| n.hostos_version != self.version).collect::<Vec<_>>()
    }

    pub async fn nodes_updated_to_the_new_version(&self) -> Vec<Node> {
        self.nodes_all
            .iter()
            .filter(|n| n.hostos_version == self.version)
            .cloned()
            .collect::<Vec<_>>()
    }

    async fn nodes_without_proposals(&self, nodes: Vec<Node>, nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>) -> Vec<Node> {
        let nodes_with_open_proposals_ids = nodes_with_open_proposals
            .into_iter()
            .flat_map(|proposal| proposal.node_ids)
            .collect::<Vec<_>>();

        nodes
            .iter()
            .cloned()
            .filter_map(|n| {
                let node_id: NodeId = NodeId::from(n.principal);
                if nodes_with_open_proposals_ids.contains(&node_id) {
                    debug!("Skipping update on node: {} because there is an open proposal for it", &node_id);
                    None
                } else {
                    Some(n)
                }
            })
            .collect()
    }

    async fn nodes_by_status(&self, nodes: Vec<Node>, nodes_health: BTreeMap<PrincipalId, Status>) -> BTreeMap<Status, Vec<Node>> {
        let nodes_by_status = nodes
            .iter()
            .cloned()
            .map(|node| (nodes_health.get(&node.principal).cloned().unwrap_or(Status::Unknown), node))
            .fold(BTreeMap::new(), |mut acc: BTreeMap<Status, Vec<Node>>, (status, node)| {
                acc.entry(status).or_default().push(node);
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
                    .filter(|&n| candidate_nodes.iter().any(|node| node.principal == n.principal))
                    .take(nodes_to_take)
                    .cloned()
                    .collect::<Vec<_>>();
                let actual_percent = nodes.len() as f32 / subnet_size as f32 * 100.0;

                if nodes.is_empty() {
                    info!("All valid nodes in the subnet: {} have been updated", subnet_id);
                    None
                } else {
                    info!("Updating {} nodes ({}%) in the subnet {}", nodes.len(), actual_percent, subnet_id,);
                    Some(nodes)
                }
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    async fn filter_nodes_in_group(&self, update_group: NodeGroupUpdate) -> anyhow::Result<Vec<Node>> {
        let filtered_nodes = self
            .grouped_nodes
            .iter()
            .filter_map(|(NodeGroup { assignment, owner }, n)| {
                if update_group.node_group.assignment == *assignment
                    && (update_group.node_group.owner == NodeOwner::All || update_group.node_group.owner == *owner)
                {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        if filtered_nodes.is_empty() {
            Err(anyhow!("No candidates selected for: {}", update_group.node_group))
        } else {
            Ok(filtered_nodes)
        }
    }

    async fn nodes_filter_in_only(&self, nodes: Vec<Node>, filter_in_features: &[String]) -> Vec<Node> {
        if filter_in_features.is_empty() {
            return nodes;
        }
        nodes
            .iter()
            .filter(|n| {
                let node = decentralization::network::Node::from(n.to_owned());
                for filt in self.only_filter.iter() {
                    if node.matches_feature_value(filt) {
                        return true;
                    }
                }
                false
            })
            .cloned()
            .collect::<Vec<_>>()
    }

    async fn nodes_filter_out_excluded(&self, nodes: Vec<Node>, excluded: &[String]) -> Vec<Node> {
        if excluded.is_empty() {
            return nodes;
        }
        nodes
            .iter()
            .filter(|n| {
                let node = decentralization::network::Node::from(n.to_owned());
                for filt in self.exclude_filter.iter() {
                    if node.matches_feature_value(filt) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect::<Vec<_>>()
    }

    async fn candidates_selection(
        &self,
        nodes_health: BTreeMap<PrincipalId, Status>,
        nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>,
        nodes_in_group: Vec<Node>,
    ) -> anyhow::Result<CandidatesSelection> {
        let nodes_by_status = self.nodes_by_status(nodes_in_group.clone(), nodes_health).await;
        info!("Fetched {} nodes by status", nodes_by_status.values().flatten().count());

        let nodes = nodes_by_status.get(&Status::Healthy).cloned().unwrap_or_default();
        info!("Found {} healthy nodes", nodes.len());
        if nodes.is_empty() {
            return Ok(CandidatesSelection::None(HostosRolloutReason::NoNodeHealthy));
        }

        let nodes = self.nodes_filter_in_only(nodes, &self.only_filter).await;
        info!("Found {} nodes that match the provided 'only' filter", nodes.len());

        let nodes = self.nodes_filter_out_excluded(nodes, &self.exclude_filter).await;
        info!("Found {} nodes that match the provided 'exclude' filter", nodes.len());

        let nodes = self.nodes_without_proposals(nodes, nodes_with_open_proposals).await;
        info!("Found {} nodes without open proposals", nodes.len());
        if nodes.is_empty() {
            return Ok(CandidatesSelection::None(HostosRolloutReason::NoNodeWithoutProposal));
        }

        let nodes = self.nodes_different_version(nodes).await;
        info!("Found {} nodes with a different version", nodes.len());

        if nodes.is_empty() {
            Ok(CandidatesSelection::None(HostosRolloutReason::AllAlreadyUpdated))
        } else {
            Ok(CandidatesSelection::Ok(nodes))
        }
    }

    #[async_recursion]
    async fn with_nodes_health_and_open_proposals(
        &self,
        nodes_health: BTreeMap<PrincipalId, Status>,
        nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>,
        update_group: NodeGroupUpdate,
    ) -> anyhow::Result<HostosRolloutResponse> {
        info!(
            "CANDIDATES SELECTION FROM {} HEALTHY NODES FOR {} {} {}",
            nodes_health
                .iter()
                .filter_map(|(principal, status)| {
                    if *status == Status::Healthy {
                        Some(principal)
                    } else {
                        None
                    }
                })
                .count(),
            update_group.maybe_number_nodes.unwrap_or_default(),
            update_group.node_group.owner,
            update_group.node_group.assignment
        );

        match update_group.node_group.assignment {
            NodeAssignment::Unassigned => {
                let unassigned_nodes = self.filter_nodes_in_group(update_group).await?;

                match self
                    .candidates_selection(nodes_health, nodes_with_open_proposals, unassigned_nodes.clone())
                    .await?
                {
                    CandidatesSelection::Ok(candidates_unassigned) => {
                        let nodes_to_take = update_group.nodes_to_take(unassigned_nodes.len());
                        let nodes_to_update = candidates_unassigned.into_iter().take(nodes_to_take).collect::<Vec<_>>();
                        info!("{} candidate nodes selected for: {}", nodes_to_update.len(), update_group.node_group);
                        Ok(HostosRolloutResponse::Ok(nodes_to_update, None))
                    }
                    CandidatesSelection::None(reason) => {
                        info!("No candidate nodes selected for: {} ==> {:?}", update_group.node_group, reason);
                        Ok(HostosRolloutResponse::None(vec![(update_group.node_group, reason)]))
                    }
                }
            }
            NodeAssignment::Assigned => {
                let assigned_nodes = self.filter_nodes_in_group(update_group).await?;
                info!("{} candidate nodes selected for: {}", assigned_nodes.len(), update_group.node_group);

                match self
                    .candidates_selection(nodes_health, nodes_with_open_proposals, assigned_nodes.clone())
                    .await?
                {
                    CandidatesSelection::Ok(candidates_assigned) => {
                        let nodes_to_update = self.take_from_subnets(candidates_assigned, update_group).await;
                        let subnets_affected = self
                            .subnets
                            .values()
                            .cloned()
                            .filter_map(|subnet| {
                                if nodes_to_update.iter().any(|node| node.subnet_id.unwrap_or_default() == subnet.principal) {
                                    Some(HostosRolloutSubnetAffected {
                                        subnet_id: subnet.principal,
                                        subnet_size: subnet.nodes.len(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<HostosRolloutSubnetAffected>>();

                        info!("{} candidate nodes selected for: {}", nodes_to_update.len(), update_group.node_group);
                        Ok(HostosRolloutResponse::Ok(nodes_to_update, Some(subnets_affected)))
                    }
                    CandidatesSelection::None(reason) => {
                        info!("No candidate nodes selected for: {} ==> {:?}", update_group.node_group, reason);
                        Ok(HostosRolloutResponse::None(vec![(update_group.node_group, reason)]))
                    }
                }
            }
            NodeAssignment::All => {
                use HostosRolloutResponse::{None, Ok};
                try_join(
                    self.with_nodes_health_and_open_proposals(
                        nodes_health.clone(),
                        nodes_with_open_proposals.clone(),
                        update_group.clone().with_assignment(NodeAssignment::Assigned),
                    ),
                    self.with_nodes_health_and_open_proposals(
                        nodes_health.clone(),
                        nodes_with_open_proposals.clone(),
                        update_group.clone().with_assignment(NodeAssignment::Unassigned),
                    ),
                )
                .await
                .map(|response| match response {
                    (Ok(assigned_nodes, subnet_affected), None(reason)) => {
                        info!("No unassigned nodes selected for: {:?} ==> {:?}", update_group.node_group, reason);
                        Ok(assigned_nodes, subnet_affected)
                    }
                    (None(reason), Ok(unassigned_nodes, _)) => {
                        info!("No assigned nodes selected for: {:?} ==> {:?}", update_group.node_group, reason);
                        Ok(unassigned_nodes, Option::None)
                    }

                    (Ok(assigned_nodes, subnet_affected), Ok(unassigned_nodes, _)) => {
                        info!(
                            "{} assigned nodes and {} unassigned nodes selected for: {}",
                            assigned_nodes.len(),
                            unassigned_nodes.len(),
                            update_group.node_group
                        );
                        Ok(assigned_nodes.into_iter().chain(unassigned_nodes).collect(), subnet_affected.clone())
                    }

                    (None(assigned_reason), None(unassigned_reason)) => {
                        info!(
                            "No candidate nodes selected for: {:?} ==> {:?} {:?}",
                            update_group.node_group, assigned_reason, unassigned_reason
                        );
                        None(assigned_reason.into_iter().chain(unassigned_reason).collect())
                    }
                })
            }
        }
    }

    /// Execute the host-os rollout operation, on the provided group of nodes.
    pub async fn execute(&self, update_group: NodeGroupUpdate) -> anyhow::Result<HostosRolloutResponse> {
        let (nodes_health, nodes_with_open_proposals) = try_join(
            health::HealthClient::new(self.network.clone()).nodes(),
            self.proposal_agent.list_open_update_nodes_hostos_versions_proposals(),
        )
        .await?;

        let nodes_on_the_new_version = self.nodes_updated_to_the_new_version().await;
        let unhealthy_nodes_on_the_new_version = nodes_on_the_new_version
            .iter()
            .filter(|n| nodes_health.get(&n.principal).cloned().unwrap_or(Status::Unknown) != Status::Healthy)
            .cloned()
            .collect::<Vec<_>>();

        info!(
            "{} nodes are already on the new version, out of those {} unhealthy",
            nodes_on_the_new_version.len(),
            unhealthy_nodes_on_the_new_version.len()
        );
        if !unhealthy_nodes_on_the_new_version.is_empty() {
            warn!(
                "\n\n ***** WARNING: Unhealthy nodes found with the new version: *****\n{}\n\n",
                unhealthy_nodes_on_the_new_version
                    .iter()
                    .map(|node| format!(
                        "{}: {:?} DC {} ({}) Node Provider {}",
                        node.principal,
                        nodes_health.get(&node.principal).cloned().unwrap_or(Status::Unknown),
                        node.operator.datacenter.as_ref().map(|dc| dc.name.as_str()).unwrap_or("-"),
                        node.label.as_deref().unwrap_or("-"),
                        node.operator.provider.name.as_deref().unwrap_or("-"),
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        self.with_nodes_health_and_open_proposals(nodes_health, nodes_with_open_proposals, update_group)
            .await
    }
}

#[cfg(test)]
pub mod test {
    use crate::operations::hostos_rollout::NodeAssignment::{Assigned, Unassigned};
    use crate::operations::hostos_rollout::NodeOwner::{Dfinity, Others};
    use ic_management_types::{Network, Node, Operator, Provider, Subnet};
    use std::net::Ipv6Addr;

    use super::*;

    #[tokio::test]
    async fn test_hostos_rollout() {
        let version_one = "ec140b74dc4fef2f4bee3fad936e315380fa5af3".to_string();
        let version_two = "e268b9807f1ab4ae65d7b29fe70a3b358d014d6a".to_string();

        let subnet_id = PrincipalId::new_subnet_test_id(0);
        let assigned_dfinity = gen_test_nodes(Some(subnet_id), 10, 0, version_one.clone(), true, false);
        let unassigned_dfinity_nodes = gen_test_nodes(None, 10, 10, version_one.clone(), true, false);
        let assigned_others_nodes = gen_test_nodes(Some(subnet_id), 10, 20, version_two.clone(), false, false);
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

        let network = Network::new("mainnet", &vec![]).await.unwrap();
        let nns_urls = network.get_nns_urls();
        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            &network,
            ProposalAgent::new(nns_urls),
            version_one.clone().as_str(),
            &[],
            &[],
        );

        let results = hostos_rollout
            .clone()
            .with_nodes_health_and_open_proposals(healthy_nodes.clone(), open_proposals.clone(), NodeGroupUpdate::new_all(Assigned, Others))
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

        let nodes_to_exclude = assigned_others_nodes.values().map(|n| n.principal.to_string()).collect::<Vec<_>>();

        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            &network,
            ProposalAgent::new(nns_urls),
            version_one.clone().as_str(),
            &[],
            &nodes_to_exclude,
        );

        let results = hostos_rollout
            .clone()
            .with_nodes_health_and_open_proposals(healthy_nodes.clone(), open_proposals.clone(), NodeGroupUpdate::new_all(Assigned, Others))
            .await
            .unwrap();

        assert!(
            matches!(results, HostosRolloutResponse::None(_)),
            "No nodes should be updated because of they have been all excluded"
        );

        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            &network,
            ProposalAgent::new(nns_urls),
            version_two.clone().as_str(),
            &[],
            &[],
        );

        let results = hostos_rollout
            .with_nodes_health_and_open_proposals(
                healthy_nodes.clone(),
                open_proposals.clone(),
                NodeGroupUpdate::new(Some(Unassigned), Some(Dfinity), NumberOfNodes::Percentage(10)),
            )
            .await
            .unwrap()
            .unwrap()
            .iter()
            .map(|n| n.principal)
            .collect::<Vec<_>>()[0];

        let want = unassigned_dfinity_nodes.clone().iter().next().map(|(_, n)| n.principal).unwrap();

        assert_eq!(results, want, "the first unassigned_dfinity_node should be updated");
    }

    fn gen_test_nodes(
        subnet_id: Option<PrincipalId>,
        num_nodes: u64,
        start_at_number: u64,
        hostos_version: String,
        dfinity_owned: bool,
        is_api_boundary_node: bool,
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
                is_api_boundary_node,
            };
            n.insert(node.principal, node);
        }
        n
    }
}
