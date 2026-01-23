//! Authentication middleware for axum

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

/// Authentication errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing authorization header")]
    MissingHeader,
    #[error("Invalid authorization format")]
    InvalidFormat,
    #[error("Session not found")]
    SessionNotFound,
    #[error("Session expired")]
    SessionExpired,
    #[error("Unauthorized")]
    Unauthorized,
}

/// A user session
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User's principal (from II)
    pub principal: String,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session expires
    pub expires_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session for a principal
    pub fn new(principal: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            principal,
            created_at: Utc::now(),
            expires_at,
        }
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Session info returned to the client
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// User's principal
    pub principal: String,
    /// When the session expires (ISO 8601)
    pub expires_at: String,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            session_id: session.id.clone(),
            principal: session.principal.clone(),
            expires_at: session.expires_at.to_rfc3339(),
        }
    }
}

/// Authentication layer for axum routes
pub struct AuthLayer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_expiration() {
        use chrono::Duration;
        
        let expired_session = Session {
            id: "test".to_string(),
            principal: "test-principal".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() - Duration::hours(1),
        };
        assert!(expired_session.is_expired());

        let valid_session = Session {
            id: "test".to_string(),
            principal: "test-principal".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        assert!(!valid_session.is_expired());
    }
}
