//! Subnet management handlers

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use slog::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::subnet::{ProposalStatus, SubnetListResponse, SubnetProposal, SubnetProposalResponse, SubnetUpgradeRequest};
use crate::state::AppState;

/// Request to create a subnet proposal
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSubnetProposalRequest {
    /// List of node IDs to include in the subnet
    pub node_ids: Vec<String>,
    /// Subnet type (default: "application")
    #[serde(default = "default_subnet_type")]
    pub subnet_type: String,
    /// Proposal title
    pub title: String,
    /// Proposal summary
    pub summary: String,
}

fn default_subnet_type() -> String {
    "application".to_string()
}

/// Request to delete a subnet
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteSubnetRequest {
    /// The subnet ID to delete
    pub subnet_id: String,
    /// Proposal title
    pub title: String,
    /// Proposal summary
    pub summary: String,
}

/// List subnets containing nodes from the configured node operator
#[utoipa::path(
    get,
    path = "/subnets/list",
    tag = "Subnets",
    responses(
        (status = 200, description = "List of subnets", body = SubnetListResponse),
        (status = 400, description = "Node operator not configured"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_subnets(State(state): State<AppState>) -> Result<Json<SubnetListResponse>, (StatusCode, String)> {
    let node_operator = state
        .config
        .node_operator
        .as_ref()
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured in config file".to_string()))?;

    // Get subnets containing operator's nodes
    let subnets = state.registry_manager.get_subnets_by_operator(&node_operator.principal_id);

    let total = subnets.len();
    Ok(Json(SubnetListResponse { subnets, total }))
}

/// Create a subnet creation proposal
#[utoipa::path(
    post,
    path = "/subnets/create",
    tag = "Subnets",
    request_body = CreateSubnetProposalRequest,
    responses(
        (status = 200, description = "Proposal created", body = SubnetProposalResponse),
        (status = 400, description = "Invalid request or node validation failed"),
        (status = 404, description = "Node not found")
    )
)]
pub async fn create_subnet_proposal(
    State(state): State<AppState>,
    Json(request): Json<CreateSubnetProposalRequest>,
) -> Result<Json<SubnetProposalResponse>, (StatusCode, String)> {
    let node_operator = state
        .config
        .node_operator
        .as_ref()
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured in config file".to_string()))?;

    // Validate that all nodes exist and belong to the configured operator
    for node_id in &request.node_ids {
        let node = state
            .registry_manager
            .get_node(node_id)
            .ok_or((StatusCode::NOT_FOUND, format!("Node {} not found", node_id)))?;

        if node.operator_id.to_string() != node_operator.principal_id {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Node {} does not belong to the configured node operator", node_id),
            ));
        }

        // Check if node is already assigned to a subnet
        if node.subnet_id.is_some() {
            return Err((StatusCode::BAD_REQUEST, format!("Node {} is already assigned to a subnet", node_id)));
        }
    }

    // Validate minimum nodes for subnet
    if request.node_ids.len() < 4 {
        return Err((StatusCode::BAD_REQUEST, "A subnet requires at least 4 nodes".to_string()));
    }

    // Create proposal (in draft state)
    let proposal = SubnetProposal {
        id: Uuid::new_v4().to_string(),
        proposal_id: None, // Not yet submitted to NNS
        status: ProposalStatus::Draft,
        node_ids: request.node_ids.clone(),
        subnet_type: request.subnet_type,
        title: request.title,
        summary: request.summary,
        created_at: chrono::Utc::now(),
    };

    // Store the proposal
    state.subnet_proposals.insert(proposal.id.clone(), proposal.clone());

    info!(state.log, "Subnet creation proposal created";
        "proposal_id" => &proposal.id,
        "node_count" => request.node_ids.len()
    );

    Ok(Json(SubnetProposalResponse {
        id: proposal.id,
        proposal_id: None,
        status: ProposalStatus::Draft,
        message: "Proposal created in draft state. Submit to NNS for voting.".to_string(),
    }))
}

/// Create a subnet deletion proposal
#[utoipa::path(
    post,
    path = "/subnets/delete",
    tag = "Subnets",
    request_body = DeleteSubnetRequest,
    responses(
        (status = 200, description = "Deletion proposal created", body = SubnetProposalResponse),
        (status = 400, description = "Node operator not configured"),
        (status = 404, description = "Subnet not found")
    )
)]
pub async fn delete_subnet_proposal(
    State(state): State<AppState>,
    Json(request): Json<DeleteSubnetRequest>,
) -> Result<Json<SubnetProposalResponse>, (StatusCode, String)> {
    let node_operator = state
        .config
        .node_operator
        .as_ref()
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured in config file".to_string()))?;

    // Verify the subnet exists and has nodes from the configured operator
    let subnets = state.registry_manager.get_subnets_by_operator(&node_operator.principal_id);

    let subnet = subnets.iter().find(|s| s.subnet_id == request.subnet_id).ok_or((
        StatusCode::NOT_FOUND,
        format!(
            "Subnet {} not found or doesn't contain nodes from the configured operator",
            request.subnet_id
        ),
    ))?;

    // Create deletion proposal
    let proposal = SubnetProposal {
        id: Uuid::new_v4().to_string(),
        proposal_id: None,
        status: ProposalStatus::Draft,
        node_ids: subnet.node_ids.clone(),
        subnet_type: "deletion".to_string(),
        title: request.title,
        summary: request.summary,
        created_at: chrono::Utc::now(),
    };

    state.subnet_proposals.insert(proposal.id.clone(), proposal.clone());

    info!(state.log, "Subnet deletion proposal created";
        "proposal_id" => &proposal.id,
        "subnet_id" => &request.subnet_id
    );

    Ok(Json(SubnetProposalResponse {
        id: proposal.id,
        proposal_id: None,
        status: ProposalStatus::Draft,
        message: format!("Deletion proposal for subnet {} created in draft state.", request.subnet_id),
    }))
}

/// Response for subnet upgrade proposal
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetUpgradeResponse {
    /// Local tracking ID
    pub id: String,
    /// Subnet ID
    pub subnet_id: String,
    /// Current GuestOS version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    /// Target GuestOS version
    pub target_version: String,
    /// Status
    pub status: ProposalStatus,
    /// Message
    pub message: String,
}

/// Create a subnet upgrade proposal to update GuestOS version
#[utoipa::path(
    post,
    path = "/subnets/upgrade",
    tag = "Subnets",
    request_body = SubnetUpgradeRequest,
    responses(
        (status = 200, description = "Upgrade proposal created", body = SubnetUpgradeResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Subnet not found")
    )
)]
pub async fn upgrade_subnet(
    State(state): State<AppState>,
    Json(request): Json<SubnetUpgradeRequest>,
) -> Result<Json<SubnetUpgradeResponse>, (StatusCode, String)> {
    // Get subnet info to validate it exists and get current version
    let subnet = state
        .registry_manager
        .get_subnet(&request.subnet_id)
        .ok_or((StatusCode::NOT_FOUND, format!("Subnet {} not found", request.subnet_id)))?;

    let current_version = subnet.replica_version.clone();

    // Check if already on target version
    if current_version.as_ref() == Some(&request.guestos_version_id) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Subnet {} is already running GuestOS version {}",
                request.subnet_id, request.guestos_version_id
            ),
        ));
    }

    // Generate default title and summary if not provided
    let title = request
        .title
        .unwrap_or_else(|| format!("Update subnet {} to GuestOS version {}", request.subnet_id, request.guestos_version_id));

    let summary = request.summary.unwrap_or_else(|| {
        format!(
            "Proposal to update subnet {} from GuestOS version {} to {}",
            request.subnet_id,
            current_version.as_deref().unwrap_or("unknown"),
            request.guestos_version_id
        )
    });

    // Create upgrade proposal (in draft state)
    let proposal_id = Uuid::new_v4().to_string();

    // Store as a subnet proposal with upgrade type
    let proposal = SubnetProposal {
        id: proposal_id.clone(),
        proposal_id: None,
        status: ProposalStatus::Draft,
        node_ids: subnet.node_ids.clone(),
        subnet_type: format!("upgrade:{}", request.guestos_version_id),
        title: title.clone(),
        summary: summary.clone(),
        created_at: chrono::Utc::now(),
    };

    state.subnet_proposals.insert(proposal_id.clone(), proposal);

    info!(state.log, "Subnet upgrade proposal created";
        "proposal_id" => &proposal_id,
        "subnet_id" => &request.subnet_id,
        "current_version" => current_version.as_deref().unwrap_or("unknown"),
        "target_version" => &request.guestos_version_id
    );

    Ok(Json(SubnetUpgradeResponse {
        id: proposal_id,
        subnet_id: request.subnet_id,
        current_version,
        target_version: request.guestos_version_id,
        status: ProposalStatus::Draft,
        message: format!(
            "Upgrade proposal created in draft state. To submit to NNS, use: ic-admin propose-to-deploy-guestos-to-all-subnet-nodes --title \"{}\" --summary \"{}\"",
            title, summary
        ),
    }))
}
