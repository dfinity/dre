use crate::{MinNakamotoCoefficients, Node, NodeGroupUpdate, Status};
use ic_base_types::PrincipalId;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
pub struct MembershipReplaceRequest {
    pub target: ReplaceTarget,
    pub heal: bool,
    pub optimize: Option<usize>,
    pub exclude: Option<Vec<String>>,
    pub only: Vec<String>,
    pub include: Option<Vec<PrincipalId>>,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
}

// impl Display for MembershipReplaceRequest
impl std::fmt::Display for MembershipReplaceRequest {
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplaceTarget {
    /// Subnet targeted for replacements
    Subnet(PrincipalId),
    /// Nodes on the same subnet that need to be replaced for other reasons
    Nodes {
        nodes: Vec<PrincipalId>,
        motivation: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct SubnetCreateRequest {
    pub size: usize,
    pub min_nakamoto_coefficients: Option<MinNakamotoCoefficients>,
    pub exclude: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
    pub include: Option<Vec<PrincipalId>>,
}

#[derive(Serialize, Deserialize)]
pub struct SubnetResizeRequest {
    pub subnet: PrincipalId,
    pub add: usize,
    pub remove: usize,
    pub exclude: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
    pub include: Option<Vec<PrincipalId>>,
}

#[derive(Serialize, Deserialize)]
pub struct HostosRolloutRequest {
    pub version: String,
    pub node_group: NodeGroupUpdate,
}

#[derive(Serialize, Deserialize)]
pub enum HostosRolloutResponse {
    Ok(Vec<Node>, Option<Vec<HostosRolloutSubnetAffected>>),
    None(HostosRolloutReason),
}

impl HostosRolloutResponse {
    pub fn unwrap(self) -> Vec<Node> {
        match self {
            HostosRolloutResponse::Ok(val, _) => val,
            _ => panic!("called `Option::unwrap()` on a `None` value"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct HostosRolloutSubnetAffected {
    pub subnet_id: PrincipalId,
    pub subnet_size: usize,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize)]
pub struct NodesRemoveRequest {
    pub no_auto: bool,
    pub extra_nodes_filter: Vec<String>,
    pub exclude: Option<Vec<String>>,
    pub motivation: String,
}

#[derive(Serialize, Deserialize)]
pub struct NodesRemoveResponse {
    pub removals: Vec<NodeRemoval>,
    pub motivation: String,
}

#[derive(Serialize, Deserialize)]
pub struct NodeRemoval {
    pub node: Node,
    pub reason: NodeRemovalReason,
}

#[derive(Serialize, Deserialize)]
pub enum NodeRemovalReason {
    Duplicates(PrincipalId),
    Unhealthy(Status),
    MatchedFilter(String),
}

impl NodeRemovalReason {
    pub fn message(&self) -> String {
        match self {
            NodeRemovalReason::Duplicates(p) => format!("Duplicates node {p}"),
            NodeRemovalReason::Unhealthy(s) => format!("Unhealthy status {s}"),
            NodeRemovalReason::MatchedFilter(f) => format!("Matched filter {f}"),
        }
    }
}
