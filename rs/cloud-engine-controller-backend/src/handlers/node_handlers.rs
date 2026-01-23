//! ICP Node handlers

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;

/// Request with session token
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

/// Request to get a specific node
#[derive(Debug, Deserialize)]
pub struct GetNodeRequest {
    pub token: String,
    pub node_id: String,
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

/// Node information response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeInfo {
    /// Node ID
    pub node_id: String,
    /// IC network name
    pub ic_name: String,
    /// Subnet ID if assigned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<String>,
    /// Data center ID
    pub dc_id: String,
    /// Node operator principal
    pub operator_id: String,
    /// Node provider principal
    pub node_provider_id: String,
    /// Whether this is an API boundary node
    pub is_api_bn: bool,
    /// IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// Domain if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// Response when listing nodes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeListResponse {
    /// List of nodes
    pub nodes: Vec<NodeInfo>,
    /// Total count
    pub total: usize,
}

/// List ICP nodes owned by the user's node operator
pub async fn list_nodes(
    State(state): State<AppState>,
    Json(request): Json<TokenRequest>,
) -> Result<Json<NodeListResponse>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let node_operator = user
        .node_operator
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured".to_string()))?;

    // Get nodes from registry
    let target_groups = state
        .registry_manager
        .get_nodes_by_operator(&node_operator.principal_id);

    let nodes: Vec<NodeInfo> = target_groups
        .into_iter()
        .map(|tg| {
            let ip_address = tg.get_ip_as_str();
            let subnet_id = tg.subnet_id.map(|s| s.to_string());
            NodeInfo {
                node_id: tg.node_id.to_string(),
                ic_name: tg.ic_name.clone(),
                subnet_id,
                dc_id: tg.dc_id.clone(),
                operator_id: tg.operator_id.to_string(),
                node_provider_id: tg.node_provider_id.to_string(),
                is_api_bn: tg.is_api_bn,
                ip_address,
                domain: tg.domain.clone(),
            }
        })
        .collect();

    let total = nodes.len();
    Ok(Json(NodeListResponse { nodes, total }))
}

/// Get details for a specific node
pub async fn get_node(
    State(state): State<AppState>,
    Json(request): Json<GetNodeRequest>,
) -> Result<Json<NodeInfo>, (StatusCode, String)> {
    let _session = get_session(&state, &request.token)?;

    let tg = state
        .registry_manager
        .get_node(&request.node_id)
        .ok_or((StatusCode::NOT_FOUND, format!("Node {} not found", request.node_id)))?;

    let ip_address = tg.get_ip_as_str();
    let subnet_id = tg.subnet_id.map(|s| s.to_string());
    Ok(Json(NodeInfo {
        node_id: tg.node_id.to_string(),
        ic_name: tg.ic_name.clone(),
        subnet_id,
        dc_id: tg.dc_id.clone(),
        operator_id: tg.operator_id.to_string(),
        node_provider_id: tg.node_provider_id.to_string(),
        is_api_bn: tg.is_api_bn,
        ip_address,
        domain: tg.domain.clone(),
    }))
}
