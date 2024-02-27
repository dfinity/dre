use anyhow::anyhow;
use async_recursion::async_recursion;
use clap::{Parser, ValueEnum};
use futures_util::future::try_join;
use ic_base_types::{NodeId, PrincipalId};
use ic_management_backend::health;
use ic_management_backend::proposal::ProposalAgent;
use ic_management_types::{
    Network, Node, Status, Subnet, UpdateNodesHostosVersionsProposal
};
use log::{debug, info};
use std::{collections::BTreeMap, fmt::Display, str::FromStr};

pub enum HostosRolloutResponse {
    Ok(Vec<Node>, Option<Vec<HostosRolloutSubnetAffected>>),
    None(Vec<(NodeGroup, HostosRolloutReason)>),
}

impl HostosRolloutResponse {
    pub fn unwrap(self) -> Vec<Node> {
        match self {
            HostosRolloutResponse::Ok(val, _) => val,
            _ => panic!("called `Option::unwrap()` on a `None` value"),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct HostosRolloutSubnetAffected {
    pub subnet_id: PrincipalId,
    pub subnet_size: usize,
}

#[derive(Eq, PartialEq)]
pub enum HostosRolloutReason {
    NoNodeHealthy,
    NoNodeWithoutProposal,
    AllAlreadyUpdated,
    NoNodeSelected,
}

impl Display for HostosRolloutReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoNodeHealthy => write!(f, "No healthy node found in the group"),
            Self::NoNodeWithoutProposal => write!(f, "No node without open proposals found in the group"),
            Self::AllAlreadyUpdated => write!(f, "All candidate nodes have been already updated"),
            Self::NoNodeSelected => write!(f, "No candidate nodes have been selected"),
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Parser, Default)]
pub enum NodeOwner {
    Dfinity,
    Others,
    #[default]
    All,
}

#[derive(ValueEnum, Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd, Default)]
pub enum NodeAssignment {
    Unassigned,
    Assigned,
    #[default]
    All,
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
impl NumberOfNodes {
    pub fn get_value(&self) -> i32 {
        match self {
            NumberOfNodes::Percentage(value) | NumberOfNodes::Absolute(value) => *value,
        }
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
    pub fn new_all(assignment: NodeAssignment, owner: NodeOwner) -> Self {
        NodeGroupUpdate {
            node_group: NodeGroup::new(assignment, owner),
            maybe_number_nodes: None,
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
            NumberOfNodes::Percentage(percent_to_update) => {
                (group_size as f32 * percent_to_update as f32 / 100.0).floor() as usize
            }
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
    pub grouped_nodes: BTreeMap<NodeGroup, Vec<Node>>,
    pub subnets: BTreeMap<PrincipalId, Subnet>,
    pub network: Network,
    pub proposal_agent: ProposalAgent,
    pub exclude: Option<Vec<PrincipalId>>,
    pub version: String,
}
impl HostosRollout {
    pub fn new(
        nodes: BTreeMap<PrincipalId, Node>,
        subnets: BTreeMap<PrincipalId, Subnet>,
        network: Network,
        proposal_agent: ProposalAgent,
        rollout_version: &str,
        nodes_filter: &Option<Vec<PrincipalId>>,
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
            grouped_nodes,
            subnets,
            network,
            proposal_agent,
            exclude: nodes_filter.clone(),
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
            .fold(
                BTreeMap::new(),
                |mut acc: BTreeMap<Status, Vec<Node>>, (status, node)| {
                    acc.entry(status).or_default().push(node);
                    acc
                },
            );

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

    async fn nodes_not_excluded(&self, nodes: Vec<Node>, excluded: &[PrincipalId]) -> Option<Vec<Node>> {
        let nodes_not_excluded = nodes
            .iter()
            .filter(|n| !excluded.contains(&n.principal))
            .cloned()
            .collect::<Vec<_>>();

        if !nodes_not_excluded.is_empty() {
            return Some(nodes_not_excluded);
        }
        None
    }

    async fn candidates_selection(
        &self,
        nodes_health: BTreeMap<PrincipalId, Status>,
        nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>,
        nodes_in_group: Vec<Node>,
    ) -> anyhow::Result<CandidatesSelection> {
        info!("Fetching candidate nodes health status");
        let nodes_by_status = self.nodes_by_status(nodes_in_group.clone(), nodes_health).await;

        info!("Selecting healthy candidate nodes");
        let nodes_healthy = match nodes_by_status.get(&Status::Healthy) {
            Some(nodes_by_status) => nodes_by_status.clone(),
            None => {
                return Ok(CandidatesSelection::None(HostosRolloutReason::NoNodeHealthy));
            }
        };

        info!("Filtering out candidate nodes with an open proposal");
        let nodes_without_proposals = match self
            .nodes_without_proposals(nodes_healthy, nodes_with_open_proposals)
            .await
        {
            Some(nodes_without_proposals) => nodes_without_proposals,
            None => {
                return Ok(CandidatesSelection::None(HostosRolloutReason::NoNodeWithoutProposal));
            }
        };

        info!("Filtering out candidate nodes already updated");
        let nodes_already_updated = match self.nodes_different_version(nodes_without_proposals).await {
            Some(nodes_different_version) => nodes_different_version,
            None => {
                return Ok(CandidatesSelection::None(HostosRolloutReason::AllAlreadyUpdated));
            }
        };

        info!("Finding not-excluded candidate nodes");
        let candidate_nodes = if let Some(excluded) = &self.exclude {
            match self.nodes_not_excluded(nodes_already_updated, excluded).await {
                Some(nodes_not_filtered) => nodes_not_filtered,
                None => {
                    return Ok(CandidatesSelection::None(HostosRolloutReason::AllAlreadyUpdated));
                }
            }
        } else {
            nodes_already_updated
        };

        Ok(CandidatesSelection::Ok(candidate_nodes))
    }
    #[async_recursion]
    async fn with_nodes_health_and_open_proposals(
        &self,
        nodes_health: BTreeMap<PrincipalId, Status>,
        nodes_with_open_proposals: Vec<UpdateNodesHostosVersionsProposal>,
        update_group: NodeGroupUpdate,
    ) -> anyhow::Result<HostosRolloutResponse> {
        info!("CANDIDATES SELECTION FOR {:?}", &update_group);

        match update_group.node_group.assignment {
            NodeAssignment::Unassigned => {
                let unassigned_nodes = self.filter_nodes_in_group(update_group).await?;

                match self
                    .candidates_selection(nodes_health, nodes_with_open_proposals, unassigned_nodes.clone())
                    .await?
                {
                    CandidatesSelection::Ok(candidates_unassigned) => {
                        let nodes_to_take = update_group.nodes_to_take(unassigned_nodes.len());
                        let nodes_to_update = candidates_unassigned
                            .into_iter()
                            .take(nodes_to_take)
                            .collect::<Vec<_>>();
                        Ok(HostosRolloutResponse::Ok(nodes_to_update, None))
                    }
                    CandidatesSelection::None(reason) => {
                        Ok(HostosRolloutResponse::None(vec![(update_group.node_group, reason)]))
                    }
                }
            }
            NodeAssignment::Assigned => {
                let assigned_nodes = self.filter_nodes_in_group(update_group).await?;

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

                        Ok(HostosRolloutResponse::Ok(nodes_to_update, Some(subnets_affected)))
                    }
                    CandidatesSelection::None(reason) => {
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
                    (Ok(assigned_nodes, subnet_affected), None(_)) => Ok(assigned_nodes, subnet_affected),
                    (None(_), Ok(unassigned_nodes, _)) => Ok(unassigned_nodes, Option::None),

                    (Ok(assigned_nodes, subnet_affected), Ok(unassigned_nodes, _)) => Ok(
                        assigned_nodes.into_iter().chain(unassigned_nodes).collect(),
                        subnet_affected.clone(),
                    ),

                    (None(assigned_reason), None(unassigned_reason)) => {
                        None(assigned_reason.into_iter().chain(unassigned_reason).collect())
                    }
                })
            }
        }
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
    use ic_management_types::{Network, Node, Operator, Provider, Subnet};
    use crate::operations::hostos_rollout::NodeOwner::{Dfinity, Others};
    use crate::operations::hostos_rollout::NodeAssignment::{Assigned, Unassigned};
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
            &None,
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

        let nodes_to_exclude = assigned_others_nodes.values().map(|n| n.principal).collect::<Vec<_>>();

        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            Network::Mainnet,
            ProposalAgent::new("https://ic0.app".to_string()),
            version_one.clone().as_str(),
            &Some(nodes_to_exclude),
        );

        let results = hostos_rollout
            .clone()
            .with_nodes_health_and_open_proposals(
                healthy_nodes.clone(),
                open_proposals.clone(),
                NodeGroupUpdate::new_all(Assigned, Others),
            )
            .await
            .unwrap();

        assert!(
            matches!(results, HostosRolloutResponse::None(_)),
            "No nodes should be updated because of they have been all excluded"
        );

        let hostos_rollout = HostosRollout::new(
            union.clone(),
            subnet.clone(),
            Network::Mainnet,
            ProposalAgent::new("https://ic0.app".to_string()),
            version_two.clone().as_str(),
            &None,
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
