//! Registry synchronization using service-discovery

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use service_discovery::{IcServiceDiscovery, IcServiceDiscoveryImpl, TargetGroup};
use service_discovery::job_types::JobType;
use slog::{Logger, info};
use thiserror::Error;
use url::Url;

use super::node_mapping::NodeMapper;
use crate::models::{IcpNodeMapping, SubnetInfo};

/// Errors from registry operations
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Failed to initialize registry: {0}")]
    InitError(String),
    #[error("Failed to sync registry: {0}")]
    SyncError(String),
    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

/// Manages the local registry copy and provides node information
pub struct RegistryManager {
    log: Logger,
    targets_dir: PathBuf,
    nns_urls: Vec<Url>,
    poll_interval: Duration,
    registry_query_timeout: Duration,
    /// Cached service discovery implementation
    service_discovery: Arc<RwLock<Option<IcServiceDiscoveryImpl>>>,
    /// Cached node data
    node_cache: Arc<RwLock<HashMap<String, TargetGroup>>>,
    /// Node mapper for IP-to-node lookups
    node_mapper: Arc<RwLock<NodeMapper>>,
}

impl RegistryManager {
    /// Create a new registry manager
    pub fn new(
        log: Logger,
        targets_dir: PathBuf,
        nns_urls: Vec<Url>,
        poll_interval: Duration,
        registry_query_timeout: Duration,
    ) -> Self {
        Self {
            log,
            targets_dir,
            nns_urls,
            poll_interval,
            registry_query_timeout,
            service_discovery: Arc::new(RwLock::new(None)),
            node_cache: Arc::new(RwLock::new(HashMap::new())),
            node_mapper: Arc::new(RwLock::new(NodeMapper::new())),
        }
    }

    /// Initialize the service discovery
    pub fn initialize(&self) -> Result<(), RegistryError> {
        // Ensure targets directory exists
        if !self.targets_dir.exists() {
            std::fs::create_dir_all(&self.targets_dir)
                .map_err(|e| RegistryError::InitError(format!("Failed to create targets dir: {}", e)))?;
        }

        let sd = IcServiceDiscoveryImpl::new(
            self.log.clone(),
            &self.targets_dir,
            self.registry_query_timeout,
        )
        .map_err(|e| RegistryError::InitError(e.to_string()))?;

        *self.service_discovery.write() = Some(sd);
        info!(self.log, "Registry manager initialized");
        Ok(())
    }

    /// Sync the registry with NNS and refresh node cache
    pub async fn sync(&self) -> Result<(), RegistryError> {
        let sd_guard = self.service_discovery.read();
        let sd = sd_guard
            .as_ref()
            .ok_or_else(|| RegistryError::InitError("Service discovery not initialized".into()))?;

        // Update registries
        sd.update_registries()
            .await
            .map_err(|e| RegistryError::SyncError(e.to_string()))?;

        // Load new ICs if any
        sd.load_new_ics(self.log.clone())
            .map_err(|e| RegistryError::SyncError(e.to_string()))?;

        // Refresh node cache
        drop(sd_guard);
        self.refresh_node_cache()?;

        info!(self.log, "Registry synced successfully");
        Ok(())
    }

    /// Refresh the node cache from service discovery
    fn refresh_node_cache(&self) -> Result<(), RegistryError> {
        let sd_guard = self.service_discovery.read();
        let sd = sd_guard
            .as_ref()
            .ok_or_else(|| RegistryError::InitError("Service discovery not initialized".into()))?;

        // Get all nodes (using Replica job type to get assigned nodes)
        let target_groups = sd
            .get_target_groups(JobType::Replica, self.log.clone())
            .map_err(|e| RegistryError::SyncError(e.to_string()))?;

        let mut cache = self.node_cache.write();
        let mut mapper = self.node_mapper.write();

        cache.clear();
        mapper.clear();

        for tg in target_groups {
            let node_id = tg.node_id.to_string();
            
            // Add to cache
            cache.insert(node_id.clone(), tg.clone());
            
            // Add to mapper
            if let Some(ip) = tg.get_ip_as_str() {
                mapper.add_mapping(ip, node_id);
            }
        }

        info!(self.log, "Node cache refreshed"; "node_count" => cache.len());
        Ok(())
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<TargetGroup> {
        self.node_cache.read().values().cloned().collect()
    }

    /// Get nodes by operator principal
    pub fn get_nodes_by_operator(&self, operator_id: &str) -> Vec<TargetGroup> {
        self.node_cache
            .read()
            .values()
            .filter(|tg| tg.operator_id.to_string() == operator_id)
            .cloned()
            .collect()
    }

    /// Get a specific node by ID
    pub fn get_node(&self, node_id: &str) -> Option<TargetGroup> {
        self.node_cache.read().get(node_id).cloned()
    }

    /// Get node by IP address
    pub fn get_node_by_ip(&self, ip: &str) -> Option<TargetGroup> {
        let mapper = self.node_mapper.read();
        mapper.get_node_id(ip).and_then(|node_id| {
            self.node_cache.read().get(&node_id).cloned()
        })
    }

    /// Map an IP address to ICP node info
    pub fn map_ip_to_node(&self, ip: &str) -> Option<IcpNodeMapping> {
        self.get_node_by_ip(ip).map(|tg| IcpNodeMapping {
            node_id: tg.node_id.to_string(),
            subnet_id: tg.subnet_id.map(|s| s.to_string()),
            dc_id: Some(tg.dc_id.clone()),
            operator_id: Some(tg.operator_id.to_string()),
        })
    }

    /// Get subnets containing nodes owned by a specific operator
    pub fn get_subnets_by_operator(&self, operator_id: &str) -> Vec<SubnetInfo> {
        let nodes = self.get_nodes_by_operator(operator_id);
        
        // Group nodes by subnet
        let mut subnet_nodes: HashMap<String, Vec<String>> = HashMap::new();
        
        for node in nodes {
            if let Some(subnet_id) = node.subnet_id {
                subnet_nodes
                    .entry(subnet_id.to_string())
                    .or_default()
                    .push(node.node_id.to_string());
            }
        }

        subnet_nodes
            .into_iter()
            .map(|(subnet_id, node_ids)| SubnetInfo {
                subnet_id,
                subnet_type: None, // Would need additional registry data
                node_count: node_ids.len(),
                node_ids,
            })
            .collect()
    }

    /// Get all subnets
    pub fn get_all_subnets(&self) -> Vec<SubnetInfo> {
        let cache = self.node_cache.read();
        
        // Group nodes by subnet
        let mut subnet_nodes: HashMap<String, Vec<String>> = HashMap::new();
        
        for tg in cache.values() {
            if let Some(subnet_id) = &tg.subnet_id {
                subnet_nodes
                    .entry(subnet_id.to_string())
                    .or_default()
                    .push(tg.node_id.to_string());
            }
        }

        subnet_nodes
            .into_iter()
            .map(|(subnet_id, node_ids)| SubnetInfo {
                subnet_id,
                subnet_type: None,
                node_count: node_ids.len(),
                node_ids,
            })
            .collect()
    }
}

impl Clone for RegistryManager {
    fn clone(&self) -> Self {
        Self {
            log: self.log.clone(),
            targets_dir: self.targets_dir.clone(),
            nns_urls: self.nns_urls.clone(),
            poll_interval: self.poll_interval,
            registry_query_timeout: self.registry_query_timeout,
            service_discovery: self.service_discovery.clone(),
            node_cache: self.node_cache.clone(),
            node_mapper: self.node_mapper.clone(),
        }
    }
}
