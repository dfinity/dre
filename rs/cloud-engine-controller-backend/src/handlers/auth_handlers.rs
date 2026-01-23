//! Authentication handlers

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use slog::info;

use crate::auth::{
    ii_delegation::{VerifyDelegationRequest, VerifyDelegationResponse, verify_delegation as verify_ii_delegation},
    middleware::Session,
};
use crate::state::AppState;

/// Request with session token
#[derive(Debug, Deserialize)]
pub struct SessionRequest {
    pub token: String,
}

/// Verify an Internet Identity delegation and create a session
pub async fn verify_delegation(
    State(state): State<AppState>,
    Json(request): Json<VerifyDelegationRequest>,
) -> Result<Json<VerifyDelegationResponse>, (StatusCode, String)> {
    // Verify the delegation chain
    match verify_ii_delegation(&request.delegation_chain, &request.session_pubkey) {
        Ok(principal) => {
            // Calculate expiration from delegation
            let expires_at = request
                .delegation_chain
                .delegations
                .last()
                .map(|d| {
                    let nanos = d.delegation.expiration;
                    let secs = (nanos / 1_000_000_000) as i64;
                    chrono::DateTime::from_timestamp(secs, 0)
                        .unwrap_or_else(|| Utc::now() + Duration::hours(24))
                })
                .unwrap_or_else(|| Utc::now() + Duration::hours(24));

            // Create a session
            let session = Session::new(principal.clone(), expires_at);
            let session_token = session.id.clone();

            // Store the session
            state.sessions.insert(session.id.clone(), session);

            // Ensure user exists
            state.get_or_create_user(&principal);

            info!(state.log, "Session created for principal"; "principal" => &principal);

            Ok(Json(VerifyDelegationResponse {
                valid: true,
                principal: Some(principal),
                session_token: Some(session_token),
                expires_at: Some(expires_at.timestamp_nanos_opt().unwrap_or(0) as u64),
                error: None,
            }))
        }
        Err(e) => {
            Ok(Json(VerifyDelegationResponse {
                valid: false,
                principal: None,
                session_token: None,
                expires_at: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Get current session information
pub async fn get_session(
    State(state): State<AppState>,
    Json(request): Json<SessionRequest>,
) -> Result<Json<crate::auth::middleware::SessionInfo>, (StatusCode, String)> {
    let session = state
        .sessions
        .get(&request.token)
        .ok_or((StatusCode::UNAUTHORIZED, "Session not found".to_string()))?;
    
    if session.is_expired() {
        state.sessions.remove(&request.token);
        return Err((StatusCode::UNAUTHORIZED, "Session expired".to_string()));
    }

    Ok(Json(crate::auth::middleware::SessionInfo::from(&*session)))
}

/// Logout (invalidate session)
pub async fn logout(
    State(state): State<AppState>,
    Json(request): Json<SessionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if state.sessions.remove(&request.token).is_some() {
        info!(state.log, "Session invalidated"; "session_id" => &request.token);
        Ok(StatusCode::OK)
    } else {
        Err((StatusCode::UNAUTHORIZED, "Session not found".to_string()))
    }
}
