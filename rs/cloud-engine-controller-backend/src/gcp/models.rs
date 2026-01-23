//! GCP API response models

use serde::{Deserialize, Serialize};

/// A GCP Compute Engine instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcpInstance {
    /// Instance ID
    pub id: String,
    /// Instance name
    pub name: String,
    /// Zone URL
    pub zone: String,
    /// Machine type URL
    pub machine_type: String,
    /// Status (RUNNING, STOPPED, etc.)
    pub status: String,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_timestamp: Option<String>,
    /// Network interfaces
    #[serde(default)]
    pub network_interfaces: Vec<GcpNetworkInterface>,
    /// Labels
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl GcpInstance {
    /// Extract zone name from zone URL
    pub fn zone_name(&self) -> String {
        self.zone
            .rsplit('/')
            .next()
            .unwrap_or(&self.zone)
            .to_string()
    }

    /// Extract machine type name from URL
    pub fn machine_type_name(&self) -> String {
        self.machine_type
            .rsplit('/')
            .next()
            .unwrap_or(&self.machine_type)
            .to_string()
    }

    /// Get internal IP address
    pub fn internal_ip(&self) -> Option<String> {
        self.network_interfaces
            .first()
            .and_then(|ni| ni.network_ip.clone())
    }

    /// Get external IP address
    pub fn external_ip(&self) -> Option<String> {
        self.network_interfaces
            .first()
            .and_then(|ni| {
                ni.access_configs
                    .as_ref()
                    .and_then(|configs| configs.first())
                    .and_then(|config| config.nat_ip.clone())
            })
    }

    /// Get IPv6 address
    pub fn ipv6_address(&self) -> Option<String> {
        self.network_interfaces
            .first()
            .and_then(|ni| ni.ipv6_address.clone())
    }
}

/// Network interface on an instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcpNetworkInterface {
    /// Network URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// Subnetwork URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnetwork: Option<String>,
    /// Internal IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_ip: Option<String>,
    /// IPv6 address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<String>,
    /// Access configurations (for external IPs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_configs: Option<Vec<GcpAccessConfig>>,
}

/// Access configuration for external IP
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcpAccessConfig {
    /// Type of access (ONE_TO_ONE_NAT)
    #[serde(rename = "type")]
    pub config_type: Option<String>,
    /// Name
    pub name: Option<String>,
    /// NAT IP (external IP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nat_ip: Option<String>,
}

/// List of instances response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcpInstanceList {
    /// List of instances
    #[serde(default)]
    pub items: Vec<GcpInstance>,
    /// Next page token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// A GCP operation (for async operations like creating/deleting VMs)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcpOperation {
    /// Operation ID
    pub id: String,
    /// Operation name
    pub name: String,
    /// Target link (the resource being operated on)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_link: Option<String>,
    /// Status (PENDING, RUNNING, DONE)
    pub status: String,
    /// Progress (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<i32>,
    /// Error if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<GcpOperationError>,
}

/// Operation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpOperationError {
    /// List of errors
    pub errors: Vec<GcpOperationErrorItem>,
}

/// Individual error item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpOperationErrorItem {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
}

/// A GCP zone
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GcpZone {
    /// Zone name
    pub name: String,
    /// Zone description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Status
    pub status: String,
    /// Region URL
    pub region: String,
}

/// Request body for creating an instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceRequest {
    /// Instance name
    pub name: String,
    /// Machine type URL
    pub machine_type: String,
    /// Disks
    pub disks: Vec<AttachedDisk>,
    /// Network interfaces
    pub network_interfaces: Vec<NetworkInterfaceConfig>,
    /// Labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
    /// Network tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Tags>,
}

/// Attached disk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedDisk {
    /// Boot disk
    pub boot: bool,
    /// Auto delete
    pub auto_delete: bool,
    /// Initialize params
    pub initialize_params: DiskInitializeParams,
}

/// Disk initialization parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInitializeParams {
    /// Disk size in GB
    pub disk_size_gb: String,
    /// Source image
    pub source_image: String,
    /// Disk type
    pub disk_type: String,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceConfig {
    /// Network URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// Subnetwork URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnetwork: Option<String>,
    /// Access configs for external IP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_configs: Option<Vec<AccessConfigRequest>>,
}

/// Access config request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessConfigRequest {
    #[serde(rename = "type")]
    pub config_type: String,
    pub name: String,
}

/// Network tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tags {
    pub items: Vec<String>,
}
