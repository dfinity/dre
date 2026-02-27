//! ICP Node handlers

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;

/// Request to get a specific node
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetNodeRequest {
    /// The node ID to look up
    pub node_id: String,
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

/// Convert registry NodeInfo to handler NodeInfo
fn convert_node(n: &crate::registry::sync::NodeInfo) -> NodeInfo {
    NodeInfo {
        node_id: n.node_id.to_string(),
        ic_name: n.ic_name.clone(),
        subnet_id: n.subnet_id.map(|s| s.to_string()),
        dc_id: n.dc_id.clone(),
        operator_id: n.operator_id.to_string(),
        node_provider_id: n.node_provider_id.to_string(),
        is_api_bn: n.is_api_bn,
        ip_address: n.get_ip_as_str(),
        domain: n.domain.clone(),
    }
}

/// List ICP nodes owned by the configured node operator
#[utoipa::path(
    get,
    path = "/nodes/list",
    tag = "Nodes",
    responses(
        (status = 200, description = "List of nodes", body = NodeListResponse),
        (status = 400, description = "Node operator not configured"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_nodes(State(state): State<AppState>) -> Result<Json<NodeListResponse>, (StatusCode, String)> {
    let node_operator = state
        .config
        .node_operator
        .as_ref()
        .ok_or((StatusCode::BAD_REQUEST, "Node operator not configured in config file".to_string()))?;

    // Get nodes from registry
    let registry_nodes = state.registry_manager.get_nodes_by_operator(&node_operator.principal_id);

    let nodes: Vec<NodeInfo> = registry_nodes.iter().map(convert_node).collect();

    let total = nodes.len();
    Ok(Json(NodeListResponse { nodes, total }))
}

/// Get details for a specific node
#[utoipa::path(
    post,
    path = "/nodes/get",
    tag = "Nodes",
    request_body = GetNodeRequest,
    responses(
        (status = 200, description = "Node details", body = NodeInfo),
        (status = 404, description = "Node not found")
    )
)]
pub async fn get_node(State(state): State<AppState>, Json(request): Json<GetNodeRequest>) -> Result<Json<NodeInfo>, (StatusCode, String)> {
    let n = state
        .registry_manager
        .get_node(&request.node_id)
        .ok_or((StatusCode::NOT_FOUND, format!("Node {} not found", request.node_id)))?;

    Ok(Json(convert_node(&n)))
}
