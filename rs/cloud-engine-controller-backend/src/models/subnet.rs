//! Subnet-related models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Information about a subnet
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetInfo {
    /// Subnet ID
    pub subnet_id: String,
    /// Subnet type (application, system, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_type: Option<String>,
    /// Current GuestOS replica version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replica_version: Option<String>,
    /// Node IDs in this subnet
    pub node_ids: Vec<String>,
    /// Number of nodes in the subnet
    pub node_count: usize,
}

/// Request to create a subnet proposal
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetProposalRequest {
    /// Node IDs to include in the new subnet
    pub node_ids: Vec<String>,
    /// Subnet type
    #[serde(default = "default_subnet_type")]
    pub subnet_type: String,
    /// Proposal title
    pub title: String,
    /// Proposal summary/motivation
    pub summary: String,
}

fn default_subnet_type() -> String {
    "application".to_string()
}

/// A subnet proposal
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetProposal {
    /// Local tracking ID
    pub id: String,
    /// NNS Proposal ID (if submitted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<u64>,
    /// Status of the proposal
    pub status: ProposalStatus,
    /// Node IDs in the proposal
    pub node_ids: Vec<String>,
    /// Subnet type
    pub subnet_type: String,
    /// Title
    pub title: String,
    /// Summary
    pub summary: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Status of a subnet proposal
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Draft, not yet submitted
    Draft,
    /// Submitted to NNS
    Submitted,
    /// Pending vote
    Pending,
    /// Accepted
    Accepted,
    /// Rejected
    Rejected,
    /// Executed
    Executed,
    /// Failed
    Failed,
}

/// Response when listing subnets
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetListResponse {
    /// List of subnets containing user's nodes
    pub subnets: Vec<SubnetInfo>,
    /// Total count
    pub total: usize,
}

/// Response when creating a proposal
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetProposalResponse {
    /// Local tracking ID
    pub id: String,
    /// NNS Proposal ID (if submitted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<u64>,
    /// Status
    pub status: ProposalStatus,
    /// Message
    pub message: String,
}

/// Request to delete a subnet (create deletion proposal)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetDeleteRequest {
    /// Proposal title
    pub title: String,
    /// Proposal summary/motivation
    pub summary: String,
}

/// Request to upgrade a subnet to a new GuestOS version
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetUpgradeRequest {
    /// Subnet ID to upgrade
    pub subnet_id: String,
    /// Target GuestOS version ID
    pub guestos_version_id: String,
    /// Proposal title (optional, will be auto-generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Proposal summary (optional, will be auto-generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}
