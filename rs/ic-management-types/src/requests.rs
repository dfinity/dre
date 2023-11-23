use crate::{MinNakamotoCoefficients, Node, Status};
use ic_base_types::PrincipalId;
use serde::{Deserialize, Serialize};

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
pub struct NodesUpdateRequest {
    pub nodes: Vec<String>,
    pub version: String,
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
