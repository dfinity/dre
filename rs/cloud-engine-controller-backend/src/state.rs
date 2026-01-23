//! Application state management

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use parking_lot::RwLock;
use slog::{Logger, info, warn};
use url::Url;

use crate::auth::Session;
use crate::gcp::GcpClient;
use crate::models::{SubnetProposal, User};
use crate::registry::RegistryManager;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Logger
    pub log: Logger,
    /// Active sessions (session_id -> Session)
    pub sessions: Arc<DashMap<String, Session>>,
    /// User store (principal -> User)
    pub users: Arc<RwLock<HashMap<String, User>>>,
    /// GCP client
    pub gcp_client: Arc<GcpClient>,
    /// Registry manager for ICP node data
    pub registry_manager: Arc<RegistryManager>,
    /// Subnet proposals (proposal_id -> SubnetProposal)
    pub subnet_proposals: Arc<DashMap<String, SubnetProposal>>,
    /// Path to persist user data
    users_state_file: Option<PathBuf>,
}

impl AppState {
    /// Create a new application state
    pub async fn new(
        log: Logger,
        targets_dir: PathBuf,
        nns_url: Url,
        poll_interval: Duration,
        registry_query_timeout: Duration,
        gcp_credentials_file: Option<PathBuf>,
        users_state_file: Option<PathBuf>,
    ) -> Self {
        // Initialize GCP client
        let gcp_client = GcpClient::new(gcp_credentials_file, log.clone()).await;

        // Initialize registry manager
        let registry_manager = RegistryManager::new(
            log.clone(),
            targets_dir,
            vec![nns_url],
            poll_interval,
            registry_query_timeout,
        );

        // Load users from state file if it exists
        let users = if let Some(ref path) = users_state_file {
            match Self::load_users(path) {
                Ok(users) => {
                    info!(log, "Loaded {} users from state file", users.len());
                    users
                }
                Err(e) => {
                    warn!(log, "Failed to load users from state file: {}", e);
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        Self {
            log,
            sessions: Arc::new(DashMap::new()),
            users: Arc::new(RwLock::new(users)),
            gcp_client: Arc::new(gcp_client),
            registry_manager: Arc::new(registry_manager),
            subnet_proposals: Arc::new(DashMap::new()),
            users_state_file,
        }
    }

    /// Get or create a user by principal
    pub fn get_or_create_user(&self, principal: &str) -> User {
        let mut users = self.users.write();
        if let Some(user) = users.get(principal) {
            user.clone()
        } else {
            let user = User::new(principal.to_string());
            users.insert(principal.to_string(), user.clone());
            self.persist_users_async();
            user
        }
    }

    /// Update a user
    pub fn update_user(&self, user: User) {
        let mut users = self.users.write();
        users.insert(user.principal.clone(), user);
        drop(users);
        self.persist_users_async();
    }

    /// Get a user by principal
    pub fn get_user(&self, principal: &str) -> Option<User> {
        let users = self.users.read();
        users.get(principal).cloned()
    }

    /// Persist users to state file asynchronously
    fn persist_users_async(&self) {
        if let Some(ref path) = self.users_state_file {
            let users = self.users.read().clone();
            let path = path.clone();
            let log = self.log.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::save_users(&path, &users) {
                    warn!(log, "Failed to save users to state file: {}", e);
                }
            });
        }
    }

    /// Load users from a JSON file
    fn load_users(path: &PathBuf) -> anyhow::Result<HashMap<String, User>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = std::fs::read_to_string(path)?;
        let users: HashMap<String, User> = serde_json::from_str(&content)?;
        Ok(users)
    }

    /// Save users to a JSON file
    fn save_users(path: &PathBuf, users: &HashMap<String, User>) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(users)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
