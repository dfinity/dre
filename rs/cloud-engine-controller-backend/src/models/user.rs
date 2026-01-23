//! User-related models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Represents a user's GCP account association
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GcpAccount {
    /// GCP project ID
    pub project_id: String,
    /// Optional service account email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_email: Option<String>,
    /// Zones the user has access to
    #[serde(default)]
    pub zones: Vec<String>,
}

/// Represents a user's ICP Node Operator information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeOperatorInfo {
    /// Principal ID of the node operator
    pub principal_id: String,
    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Data center IDs owned by this operator
    #[serde(default)]
    pub dc_ids: Vec<String>,
}

/// A user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User's Internet Identity principal
    pub principal: String,
    /// Associated GCP account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_account: Option<GcpAccount>,
    /// Associated node operator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_operator: Option<NodeOperatorInfo>,
    /// When the user was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the user was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// Create a new user with the given principal
    pub fn new(principal: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            principal,
            gcp_account: None,
            node_operator: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// User profile returned by the API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    /// User's Internet Identity principal
    pub principal: String,
    /// Associated GCP account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_account: Option<GcpAccount>,
    /// Associated node operator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_operator: Option<NodeOperatorInfo>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            principal: user.principal,
            gcp_account: user.gcp_account,
            node_operator: user.node_operator,
        }
    }
}

/// Request to set GCP account
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetGcpAccountRequest {
    /// GCP project ID
    pub project_id: String,
    /// Optional service account email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_email: Option<String>,
    /// Zones to monitor
    #[serde(default)]
    pub zones: Vec<String>,
}

/// Request to set node operator
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetNodeOperatorRequest {
    /// Principal ID of the node operator
    pub principal_id: String,
    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}
