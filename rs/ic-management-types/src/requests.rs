use crate::{MinNakamotoCoefficients, Node, Status};
use ic_base_types::PrincipalId;
use serde::{Deserialize, Serialize};

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
