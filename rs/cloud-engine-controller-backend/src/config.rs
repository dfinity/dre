//! Application configuration loaded from filesystem
//!
//! This module provides configuration for GCP settings that are loaded from a JSON file.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use utoipa::ToSchema;

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// GCP configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GcpConfig {
    /// GCP project ID
    pub project_id: String,
    /// Zones to operate in
    #[serde(default = "default_zones")]
    pub zones: Vec<String>,
}

fn default_zones() -> Vec<String> {
    vec![
        "us-central1-a".to_string(),
        "us-central1-b".to_string(),
        "europe-west1-b".to_string(),
    ]
}

/// Node operator configuration (optional)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeOperatorConfig {
    /// Principal ID of the node operator
    pub principal_id: String,
    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Application configuration loaded from a JSON file
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppConfig {
    /// GCP configuration
    pub gcp: GcpConfig,
    /// Optional node operator configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_operator: Option<NodeOperatorConfig>,
}

impl AppConfig {
    /// Load configuration from a JSON file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        if self.gcp.project_id.is_empty() {
            return Err(ConfigError::MissingField("gcp.project_id".to_string()));
        }
        if self.gcp.zones.is_empty() {
            return Err(ConfigError::MissingField("gcp.zones".to_string()));
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gcp: GcpConfig {
                project_id: String::new(),
                zones: default_zones(),
            },
            node_operator: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let config_json = r#"{
            "gcp": {
                "project_id": "my-project",
                "zones": ["us-central1-a", "us-central1-b"]
            },
            "node_operator": {
                "principal_id": "aaaaa-aa",
                "display_name": "Test Operator"
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_json.as_bytes()).unwrap();

        let config = AppConfig::load(file.path()).unwrap();
        assert_eq!(config.gcp.project_id, "my-project");
        assert_eq!(config.gcp.zones.len(), 2);
        assert!(config.node_operator.is_some());
    }

    #[test]
    fn test_load_minimal_config() {
        let config_json = r#"{
            "gcp": {
                "project_id": "my-project"
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_json.as_bytes()).unwrap();

        let config = AppConfig::load(file.path()).unwrap();
        assert_eq!(config.gcp.project_id, "my-project");
        // Default zones should be applied
        assert!(!config.gcp.zones.is_empty());
    }

    #[test]
    fn test_invalid_config_missing_project() {
        let config_json = r#"{
            "gcp": {
                "project_id": "",
                "zones": ["us-central1-a"]
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_json.as_bytes()).unwrap();

        let result = AppConfig::load(file.path());
        assert!(result.is_err());
    }
}
