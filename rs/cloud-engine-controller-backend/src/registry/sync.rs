//! Registry synchronization using LocalRegistry directly

use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::TryFutureExt;
use ic_interfaces_registry::{RegistryClient, ZERO_REGISTRY_VERSION};
use ic_protobuf::registry::crypto::v1::PublicKey;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_registry_client::client::{RegistryVersion, ThresholdSigPublicKey};
use ic_registry_client_fake::FakeRegistryClient;
use ic_registry_client_helpers::{
    api_boundary_node::ApiBoundaryNodeRegistry,
    node::{NodeId, NodeRegistry, SubnetId},
    node_operator::{NodeOperatorRegistry, PrincipalId},
    subnet::{SubnetListRegistry, SubnetTransportRegistry},
};
use ic_registry_common_proto::pb::local_store::v1::{
    ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType,
};
use ic_registry_keys::{make_crypto_threshold_signing_pubkey_key, ROOT_SUBNET_ID_KEY};
use ic_registry_local_registry::LocalRegistry;
use ic_registry_local_store::{Changelog, ChangelogEntry, KeyMutation, LocalStoreImpl};
use ic_registry_nns_data_provider::registry::RegistryCanister;
use parking_lot::RwLock;
use prost::Message;
use serde::{Deserialize, Serialize};
use slog::{Logger, debug, error, info, warn};
use thiserror::Error;
use tokio::sync::RwLock as TokioRwLock;
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

/// Node information from the registry
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: NodeId,
    pub ic_name: String,
    pub targets: BTreeSet<SocketAddr>,
    pub subnet_id: Option<SubnetId>,
    pub dc_id: String,
    pub operator_id: PrincipalId,
    pub node_provider_id: PrincipalId,
    pub is_api_bn: bool,
    pub domain: Option<String>,
}

impl NodeInfo {
    pub fn get_ip_as_str(&self) -> Option<String> {
        self.targets.iter().next().map(|addr| {
            let addr_str = addr.to_string();
            // Extract IP from [ip]:port format
            if addr_str.starts_with('[') {
                if let Some(end) = addr_str.find(']') {
                    return addr_str[1..end].to_string();
                }
            }
            addr.ip().to_string()
        })
    }
}

/// Manages the local registry copy and provides node information
pub struct RegistryManager {
    log: Logger,
    targets_dir: PathBuf,
    nns_urls: Vec<Url>,
    registry_query_timeout: Duration,
    /// Local registry instance (uses tokio RwLock for async-safe access)
    local_registry: Arc<TokioRwLock<Option<LocalRegistry>>>,
    /// IC name for this registry
    ic_name: String,
    /// Cached node data
    node_cache: Arc<RwLock<HashMap<String, NodeInfo>>>,
    /// Node mapper for IP-to-node lookups
    node_mapper: Arc<RwLock<NodeMapper>>,
}

impl RegistryManager {
    /// Create a new registry manager
    pub fn new(
        log: Logger,
        targets_dir: PathBuf,
        nns_urls: Vec<Url>,
        _poll_interval: Duration,
        registry_query_timeout: Duration,
    ) -> Self {
        Self {
            log,
            targets_dir,
            nns_urls,
            registry_query_timeout,
            local_registry: Arc::new(TokioRwLock::new(None)),
            ic_name: "mainnet".to_string(),
            node_cache: Arc::new(RwLock::new(HashMap::new())),
            node_mapper: Arc::new(RwLock::new(NodeMapper::new())),
        }
    }

    /// Initialize the local registry, bootstrapping from NNS if needed
    pub async fn initialize(&self) -> Result<(), RegistryError> {
        // Ensure targets directory exists
        if !self.targets_dir.exists() {
            std::fs::create_dir_all(&self.targets_dir).map_err(|e| RegistryError::InitError(format!("Failed to create targets dir: {}", e)))?;
        }

        // Check if we need to bootstrap the registry
        let needs_bootstrap = self.check_needs_bootstrap();
        
        if needs_bootstrap {
            info!(self.log, "Registry is empty, bootstrapping from NNS...");
            self.bootstrap_from_nns().await?;
            info!(self.log, "Registry bootstrap completed");
        }

        // Create local registry
        let registry = LocalRegistry::new(&self.targets_dir, self.registry_query_timeout).map_err(|e| RegistryError::InitError(e.to_string()))?;

        *self.local_registry.write().await = Some(registry);
        info!(self.log, "Registry manager initialized"; "path" => %self.targets_dir.display());
        Ok(())
    }

    /// Check if the registry needs to be bootstrapped (empty or doesn't exist)
    fn check_needs_bootstrap(&self) -> bool {
        if !self.targets_dir.exists() {
            return true;
        }

        // Try to create a FakeRegistryClient to check the version
        let local_store = Arc::new(LocalStoreImpl::new(self.targets_dir.clone()));
        let registry_cache = FakeRegistryClient::new(local_store);
        registry_cache.update_to_latest_version();
        let version = registry_cache.get_latest_version();

        version == ZERO_REGISTRY_VERSION
    }

    /// Bootstrap the registry by fetching all versions from NNS
    async fn bootstrap_from_nns(&self) -> Result<(), RegistryError> {
        let local_store = Arc::new(LocalStoreImpl::new(self.targets_dir.clone()));
        let registry_canister = RegistryCanister::new(self.nns_urls.clone());

        // Get current local version
        let mut latest_version = if !Path::new(&self.targets_dir).exists() {
            ZERO_REGISTRY_VERSION
        } else {
            let registry_cache = FakeRegistryClient::new(local_store.clone());
            registry_cache.update_to_latest_version();
            registry_cache.get_latest_version()
        };

        debug!(self.log, "Bootstrapping registry from version"; "version" => latest_version.get());

        // Get NNS public key for verification
        let nns_public_key = self.get_nns_public_key(&registry_canister).await?;

        let mut updates = vec![];

        loop {
            // Check if we're up to date
            let remote_version = registry_canister
                .get_latest_version()
                .await
                .map_err(|e| RegistryError::SyncError(format!("Failed to get latest registry version: {}", e)))?;

            if latest_version.get() >= remote_version {
                debug!(self.log, "Registry is up to date"; "version" => latest_version.get());
                break;
            }

            info!(self.log, "Fetching registry updates"; 
                "local_version" => latest_version.get(), 
                "remote_version" => remote_version
            );

            // Fetch changes since our version
            let (mut records, _, _) = registry_canister
                .get_certified_changes_since(latest_version.get(), &nns_public_key)
                .await
                .map_err(|e| RegistryError::SyncError(format!("Failed to get registry changes: {}", e)))?;

            records.sort_by_key(|r| r.version);

            let changelog = records.iter().fold(Changelog::default(), |mut cl, r| {
                let rel_version = (r.version - latest_version).get();
                if cl.len() < rel_version as usize {
                    cl.push(ChangelogEntry::default());
                }
                cl.last_mut().unwrap().push(KeyMutation {
                    key: r.key.clone(),
                    value: r.value.clone(),
                });
                cl
            });

            let versions_count = changelog.len();

            for (i, ce) in changelog.into_iter().enumerate() {
                let v = RegistryVersion::from(i as u64 + 1 + latest_version.get());
                let local_registry_path = self.targets_dir.clone();
                updates.push(async move {
                    let path_str = format!("{:016x}.pb", v.get());
                    let v_path = &[
                        &path_str[0..10],
                        &path_str[10..12],
                        &path_str[12..14],
                        &path_str[14..19],
                    ]
                    .iter()
                    .collect::<PathBuf>();

                    let path = local_registry_path.join(v_path.as_path());
                    let parent = path.parent().unwrap().to_path_buf();
                    tokio::fs::create_dir_all(parent)
                        .and_then(|_| async {
                            tokio::fs::write(
                                &path,
                                PbChangelogEntry {
                                    key_mutations: ce
                                        .iter()
                                        .map(|km| {
                                            let mutation_type = if km.value.is_some() {
                                                MutationType::Set as i32
                                            } else {
                                                MutationType::Unset as i32
                                            };
                                            PbKeyMutation {
                                                key: km.key.clone(),
                                                value: km.value.clone().unwrap_or_default(),
                                                mutation_type,
                                            }
                                        })
                                        .collect(),
                                }
                                .encode_to_vec(),
                            )
                            .await
                        })
                        .await
                });
            }

            latest_version = latest_version.add(RegistryVersion::new(versions_count as u64));
            debug!(self.log, "Bootstrap reached version"; "version" => latest_version.get());
        }

        // Wait for all writes to complete
        futures::future::join_all(updates).await;
        info!(self.log, "Bootstrap completed"; "final_version" => latest_version.get());

        Ok(())
    }

    /// Get the NNS public key for verifying registry updates
    async fn get_nns_public_key(&self, registry_canister: &RegistryCanister) -> Result<ThresholdSigPublicKey, RegistryError> {
        let (nns_subnet_id_vec, _) = registry_canister
            .get_value(ROOT_SUBNET_ID_KEY.as_bytes().to_vec(), None)
            .await
            .map_err(|e| RegistryError::SyncError(format!("Failed to get root subnet: {}", e)))?;

        let nns_subnet_id = ic_protobuf::types::v1::SubnetId::decode(nns_subnet_id_vec.as_slice())
            .map_err(|e| RegistryError::SyncError(format!("Failed to decode subnet ID: {}", e)))?;

        let (nns_pub_key_vec, _) = registry_canister
            .get_value(
                make_crypto_threshold_signing_pubkey_key(SubnetId::new(
                    PrincipalId::try_from(nns_subnet_id.principal_id.unwrap().raw).unwrap(),
                ))
                .as_bytes()
                .to_vec(),
                None,
            )
            .await
            .map_err(|e| RegistryError::SyncError(format!("Failed to get public key: {}", e)))?;

        Ok(ThresholdSigPublicKey::try_from(
            PublicKey::decode(nns_pub_key_vec.as_slice()).expect("invalid public key"),
        )
        .expect("failed to create threshold sig public key"))
    }

    /// Sync the registry with NNS and refresh node cache
    pub async fn sync(&self) -> Result<(), RegistryError> {
        // Use tokio RwLock which is safe to hold across await points
        let registry_guard = self.local_registry.read().await;
        let registry = registry_guard
            .as_ref()
            .ok_or_else(|| RegistryError::InitError("Registry not initialized".into()))?;

        // Sync with NNS
        registry.sync_with_nns().await.map_err(|e| RegistryError::SyncError(e.to_string()))?;

        // Drop the guard before refreshing cache (which needs its own lock)
        drop(registry_guard);

        // Refresh node cache
        self.refresh_node_cache().await?;

        info!(self.log, "Registry synced successfully");
        Ok(())
    }

    /// Refresh the node cache from the local registry
    async fn refresh_node_cache(&self) -> Result<(), RegistryError> {
        let registry_guard = self.local_registry.read().await;
        let registry = registry_guard
            .as_ref()
            .ok_or_else(|| RegistryError::InitError("Registry not initialized".into()))?;

        let nodes = self.get_nodes_from_registry(registry)?;

        let mut cache = self.node_cache.write();
        let mut mapper = self.node_mapper.write();

        cache.clear();
        mapper.clear();

        for node in nodes {
            let node_id_str = node.node_id.to_string();

            // Add to mapper
            if let Some(ip) = node.get_ip_as_str() {
                mapper.add_mapping(ip, node_id_str.clone());
            }

            // Add to cache
            cache.insert(node_id_str, node);
        }

        info!(self.log, "Node cache refreshed"; "node_count" => cache.len());
        Ok(())
    }

    /// Extract node information from the registry
    fn get_nodes_from_registry(&self, registry: &LocalRegistry) -> Result<Vec<NodeInfo>, RegistryError> {
        let latest_version = registry.get_latest_version();
        let mut nodes = Vec::new();

        // Get all node IDs
        let all_node_ids: BTreeSet<_> = registry
            .get_node_ids(latest_version)
            .map_err(|e| RegistryError::SyncError(format!("Failed to get node IDs: {}", e)))?
            .into_iter()
            .collect();

        let mut unassigned_node_ids = all_node_ids.clone();

        // Get subnet IDs
        let subnet_ids = registry
            .get_subnet_ids(latest_version)
            .map_err(|e| RegistryError::SyncError(format!("Failed to get subnet IDs: {}", e)))?
            .unwrap_or_default();

        // Get API boundary nodes
        let api_bns: BTreeSet<_> = registry
            .get_api_boundary_node_ids(latest_version)
            .unwrap_or_default()
            .into_iter()
            .collect();

        // Process nodes in subnets
        for subnet_id in subnet_ids {
            let subnet_node_records = match registry.get_subnet_node_records(subnet_id, latest_version) {
                Ok(Some(records)) => records,
                Ok(None) => continue,
                Err(e) => {
                    warn!(self.log, "Error fetching subnet nodes"; "subnet_id" => %subnet_id, "error" => %e);
                    continue;
                }
            };

            for (node_id, node_record) in subnet_node_records {
                if let Some(node_info) = self.create_node_info(
                    registry,
                    node_id,
                    node_record,
                    Some(subnet_id),
                    api_bns.contains(&node_id),
                    latest_version,
                ) {
                    nodes.push(node_info);
                }
                unassigned_node_ids.remove(&node_id);
            }
        }

        // Process unassigned nodes
        for node_id in unassigned_node_ids {
            let node_record = match registry.get_node_record(node_id, latest_version) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(e) => {
                    warn!(self.log, "Error fetching node record"; "node_id" => %node_id, "error" => %e);
                    continue;
                }
            };

            if let Some(node_info) = self.create_node_info(registry, node_id, node_record, None, api_bns.contains(&node_id), latest_version) {
                nodes.push(node_info);
            }
        }

        Ok(nodes)
    }

    /// Create NodeInfo from registry data
    fn create_node_info(
        &self,
        registry: &LocalRegistry,
        node_id: NodeId,
        node_record: NodeRecord,
        subnet_id: Option<SubnetId>,
        is_api_bn: bool,
        version: ic_registry_client::client::RegistryVersion,
    ) -> Option<NodeInfo> {
        // Extract socket address from node record
        let http_endpoint = node_record.http.as_ref()?;
        let ip_addr: std::net::IpAddr = http_endpoint.ip_addr.parse().ok()?;

        // Skip bogus entries with 0.0.0.0
        if ip_addr.is_unspecified() {
            return None;
        }

        let socket_addr = SocketAddr::new(ip_addr, 9090);

        // Get operator info
        let operator_id = PrincipalId::try_from(node_record.node_operator_id.clone()).unwrap_or_default();

        let node_operator = registry.get_node_operator_record(operator_id, version).ok().flatten().unwrap_or_default();

        Some(NodeInfo {
            node_id,
            ic_name: self.ic_name.clone(),
            targets: vec![socket_addr].into_iter().collect(),
            subnet_id,
            dc_id: node_operator.dc_id,
            operator_id,
            node_provider_id: PrincipalId::try_from(node_operator.node_provider_principal_id).unwrap_or_default(),
            is_api_bn,
            domain: node_record.domain,
        })
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<NodeInfo> {
        self.node_cache.read().values().cloned().collect()
    }

    /// Get nodes by operator principal
    pub fn get_nodes_by_operator(&self, operator_id: &str) -> Vec<NodeInfo> {
        self.node_cache
            .read()
            .values()
            .filter(|n| n.operator_id.to_string() == operator_id)
            .cloned()
            .collect()
    }

    /// Get a specific node by ID
    pub fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        self.node_cache.read().get(node_id).cloned()
    }

    /// Get node by IP address
    pub fn get_node_by_ip(&self, ip: &str) -> Option<NodeInfo> {
        let mapper = self.node_mapper.read();
        mapper.get_node_id(ip).and_then(|node_id| self.node_cache.read().get(&node_id).cloned())
    }

    /// Map an IP address to ICP node info
    pub fn map_ip_to_node(&self, ip: &str) -> Option<IcpNodeMapping> {
        self.get_node_by_ip(ip).map(|n| IcpNodeMapping {
            node_id: n.node_id.to_string(),
            subnet_id: n.subnet_id.map(|s| s.to_string()),
            dc_id: Some(n.dc_id.clone()),
            operator_id: Some(n.operator_id.to_string()),
        })
    }

    /// Get subnets containing nodes owned by a specific operator
    pub fn get_subnets_by_operator(&self, operator_id: &str) -> Vec<SubnetInfo> {
        let nodes = self.get_nodes_by_operator(operator_id);

        // Group nodes by subnet
        let mut subnet_nodes: HashMap<String, Vec<String>> = HashMap::new();

        for node in nodes {
            if let Some(subnet_id) = node.subnet_id {
                subnet_nodes.entry(subnet_id.to_string()).or_default().push(node.node_id.to_string());
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

    /// Get all subnets
    pub fn get_all_subnets(&self) -> Vec<SubnetInfo> {
        let cache = self.node_cache.read();

        // Group nodes by subnet
        let mut subnet_nodes: HashMap<String, Vec<String>> = HashMap::new();

        for n in cache.values() {
            if let Some(subnet_id) = &n.subnet_id {
                subnet_nodes.entry(subnet_id.to_string()).or_default().push(n.node_id.to_string());
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
            registry_query_timeout: self.registry_query_timeout,
            local_registry: self.local_registry.clone(),
            ic_name: self.ic_name.clone(),
            node_cache: self.node_cache.clone(),
            node_mapper: self.node_mapper.clone(),
        }
    }
}
