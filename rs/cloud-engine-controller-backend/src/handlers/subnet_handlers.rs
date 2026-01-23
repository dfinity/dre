//! Subnet management handlers

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use slog::info;
use uuid::Uuid;

use crate::models::subnet::{
    ProposalStatus, SubnetListResponse, SubnetProposal,
    SubnetProposalResponse,
};
use crate::state::AppState;

/// Request with session token
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

/// Request to create a subnet proposal
#[derive(Debug, Deserialize)]
pub struct CreateSubnetProposalRequest {
    pub token: String,
    pub node_ids: Vec<String>,
    #[serde(default = "default_subnet_type")]
    pub subnet_type: String,
    pub title: String,
    pub summary: String,
}

fn default_subnet_type() -> String {
    "application".to_string()
}

/// Request to delete a subnet
#[derive(Debug, Deserialize)]
pub struct DeleteSubnetRequest {
    pub token: String,
    pub subnet_id: String,
    pub title: String,
    pub summary: String,
}

/// Helper to get session from token
fn get_session(state: &AppState, token: &str) -> Result<crate::auth::Session, (StatusCode, String)> {
    let session = state
        .sessions
        .get(token)
        .ok_or((StatusCode::UNAUTHORIZED, "Session not found".to_string()))?;
    
    if session.is_expired() {
        state.sessions.remove(token);
        return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
    }
    
    Ok(session.clone())
}

/// List subnets containing the user's nodes
pub async fn list_subnets(
    State(state): State<AppState>,
    Json(request): Json<TokenRequest>,
) -> Result<Json<SubnetListResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let node_operator = user
        .node_operator
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured".to_string()))?;

    // Get subnets containing user's nodes
    let subnets = state
        .registry_manager
        .get_subnets_by_operator(&node_operator.principal_id);

    let total = subnets.len();
    Ok(Json(SubnetListResponse { subnets, total }))
}

/// Create a subnet creation proposal
pub async fn create_subnet_proposal(
    State(state): State<AppState>,
    Json(request): Json<CreateSubnetProposalRequest>,
) -> Result<Json<SubnetProposalResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let node_operator = user
        .node_operator
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured".to_string()))?;

    // Validate that all nodes exist and belong to the user
    for node_id in &request.node_ids {
        let node = state
            .registry_manager
            .get_node(node_id)
            .ok_or((StatusCode::NOT_FOUND, format!("Node {} not found", node_id)))?;

        if node.operator_id.to_string() != node_operator.principal_id {
            return Err((StatusCode::BAD_REQUEST, format!(
                "Node {} does not belong to your node operator",
                node_id
            )));
        }

        // Check if node is already assigned to a subnet
        if node.subnet_id.is_some() {
            return Err((StatusCode::BAD_REQUEST, format!(
                "Node {} is already assigned to a subnet",
                node_id
            )));
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
    state
        .subnet_proposals
        .insert(proposal.id.clone(), proposal.clone());

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
pub async fn delete_subnet_proposal(
    State(state): State<AppState>,
    Json(request): Json<DeleteSubnetRequest>,
) -> Result<Json<SubnetProposalResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let node_operator = user
        .node_operator
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured".to_string()))?;

    // Verify the subnet exists and user has nodes in it
    let subnets = state
        .registry_manager
        .get_subnets_by_operator(&node_operator.principal_id);

    let subnet = subnets
        .iter()
        .find(|s| s.subnet_id == request.subnet_id)
        .ok_or((StatusCode::NOT_FOUND, format!(
            "Subnet {} not found or you don't have nodes in it",
            request.subnet_id
        )))?;

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

    state
        .subnet_proposals
        .insert(proposal.id.clone(), proposal.clone());

    info!(state.log, "Subnet deletion proposal created";
        "proposal_id" => &proposal.id,
        "subnet_id" => &request.subnet_id
    );

    Ok(Json(SubnetProposalResponse {
        id: proposal.id,
        proposal_id: None,
        status: ProposalStatus::Draft,
        message: format!(
            "Deletion proposal for subnet {} created in draft state.",
            request.subnet_id
        ),
    }))
}
