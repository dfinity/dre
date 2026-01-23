//! GCP credentials management

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use slog::{Logger, info, warn};
use thiserror::Error;

/// Errors related to GCP credentials
#[derive(Debug, Error)]
pub enum CredentialsError {
    #[error("Failed to load service account: {0}")]
    ServiceAccountLoad(String),
    #[error("Failed to initialize authentication: {0}")]
    AuthInit(String),
    #[error("Failed to get access token: {0}")]
    TokenError(String),
}

/// GCP credentials manager
/// Uses the gcp_auth crate's provider() function to get tokens
pub struct GcpCredentials {
    provider: Arc<RwLock<Option<Arc<dyn gcp_auth::TokenProvider>>>>,
    log: Logger,
}

impl GcpCredentials {
    /// Create new credentials from default provider chain
    /// This will try (in order):
    /// 1. GOOGLE_APPLICATION_CREDENTIALS environment variable
    /// 2. Application Default Credentials file
    /// 3. GCE metadata service
    /// 4. gcloud CLI
    pub async fn new(credentials_file: Option<PathBuf>, log: Logger) -> Self {
        // If a specific credentials file is provided, set the environment variable
        if let Some(path) = credentials_file {
            if path.exists() {
                // SAFETY: We're setting an environment variable at startup, before any threads are spawned
                unsafe {
                    std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", path.to_string_lossy().to_string());
                }
                info!(log, "Set GOOGLE_APPLICATION_CREDENTIALS"; "path" => path.to_string_lossy().to_string());
            } else {
                warn!(log, "Credentials file does not exist"; "path" => path.to_string_lossy().to_string());
            }
        }

        let provider = match gcp_auth::provider().await {
            Ok(p) => {
                info!(log, "GCP authentication provider initialized");
                Some(p)
            }
            Err(e) => {
                warn!(log, "Failed to initialize GCP auth provider, GCP features will be unavailable: {}", e);
                None
            }
        };

        Self {
            provider: Arc::new(RwLock::new(provider)),
            log,
        }
    }

    /// Get an access token for the Compute Engine API
    pub async fn get_token(&self) -> Result<String, CredentialsError> {
        // Clone the provider to avoid holding the lock across await
        let provider = {
            let guard = self.provider.read();
            guard.clone()
        };
        
        let provider = provider
            .ok_or_else(|| CredentialsError::AuthInit("No authentication provider available".into()))?;

        let scopes = &["https://www.googleapis.com/auth/compute"];
        let token = provider
            .token(scopes)
            .await
            .map_err(|e| CredentialsError::TokenError(e.to_string()))?;

        Ok(token.as_str().to_string())
    }

    /// Check if credentials are available
    pub fn is_available(&self) -> bool {
        self.provider.read().is_some()
    }
}

impl Clone for GcpCredentials {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            log: self.log.clone(),
        }
    }
}
