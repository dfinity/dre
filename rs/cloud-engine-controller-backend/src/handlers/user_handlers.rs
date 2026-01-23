//! User management handlers

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use slog::info;

use crate::models::user::{
    GcpAccount, NodeOperatorInfo, UserProfile,
};
use crate::state::AppState;

/// Request with session token
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

/// Request to set GCP account
#[derive(Debug, Deserialize)]
pub struct SetGcpAccountRequest {
    pub token: String,
    pub project_id: String,
    pub service_account_email: Option<String>,
    #[serde(default)]
    pub zones: Vec<String>,
}

/// Request to set node operator
#[derive(Debug, Deserialize)]
pub struct SetNodeOperatorRequest {
    pub token: String,
    pub principal_id: String,
    pub display_name: Option<String>,
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

/// Get the current user's profile
pub async fn get_profile(
    State(state): State<AppState>,
    Json(request): Json<TokenRequest>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    let user = state
        .get_user(&session.principal)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok(Json(UserProfile::from(user)))
}

/// Set/update the user's GCP account association
pub async fn set_gcp_account(
    State(state): State<AppState>,
    Json(request): Json<SetGcpAccountRequest>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    // Validate project ID format (basic validation)
    if request.project_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Project ID cannot be empty".to_string()));
    }

    let mut user = state.get_or_create_user(&session.principal);

    user.gcp_account = Some(GcpAccount {
        project_id: request.project_id,
        service_account_email: request.service_account_email,
        zones: if request.zones.is_empty() {
            // Default zones
            vec![
                "us-central1-a".to_string(),
                "us-central1-b".to_string(),
                "europe-west1-b".to_string(),
            ]
        } else {
            request.zones
        },
    });
    user.updated_at = chrono::Utc::now();

    state.update_user(user.clone());

    info!(state.log, "GCP account updated for user"; "principal" => &session.principal);

    Ok(Json(UserProfile::from(user)))
}

/// Set/update the user's node operator association
pub async fn set_node_operator(
    State(state): State<AppState>,
    Json(request): Json<SetNodeOperatorRequest>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let session = get_session(&state, &request.token)?;

    // Validate principal ID format (basic validation)
    if request.principal_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Principal ID cannot be empty".to_string()));
    }

    let mut user = state.get_or_create_user(&session.principal);

    // Get DC IDs from registry if the node operator exists
    let dc_ids = state
        .registry_manager
        .get_nodes_by_operator(&request.principal_id)
        .iter()
        .map(|tg| tg.dc_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    user.node_operator = Some(NodeOperatorInfo {
        principal_id: request.principal_id,
        display_name: request.display_name,
        dc_ids,
    });
    user.updated_at = chrono::Utc::now();

    state.update_user(user.clone());

    info!(state.log, "Node operator updated for user"; "principal" => &session.principal);

    Ok(Json(UserProfile::from(user)))
}
