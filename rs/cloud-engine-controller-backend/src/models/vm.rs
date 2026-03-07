//! VM-related models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Status of a VM
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VmStatus {
    /// VM is running
    Running,
    /// VM is stopped
    Stopped,
    /// VM is being provisioned
    Provisioning,
    /// VM is being staged
    Staging,
    /// VM is suspended
    Suspended,
    /// VM is being terminated
    Terminating,
    /// VM status is unknown
    Unknown,
}

impl Default for VmStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<&str> for VmStatus {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "RUNNING" => Self::Running,
            "STOPPED" => Self::Stopped,
            "PROVISIONING" => Self::Provisioning,
            "STAGING" => Self::Staging,
            "SUSPENDED" => Self::Suspended,
            "TERMINATING" => Self::Terminating,
            _ => Self::Unknown,
        }
    }
}

/// Mapping from a VM to an ICP node
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IcpNodeMapping {
    /// ICP Node ID
    pub node_id: String,
    /// Subnet ID if the node is assigned to a subnet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<String>,
    /// Data center ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dc_id: Option<String>,
    /// Node operator principal ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,
}

/// Represents a GCP VM instance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Vm {
    /// GCP instance ID
    pub id: String,
    /// Instance name
    pub name: String,
    /// GCP zone
    pub zone: String,
    /// Machine type (e.g., n2-standard-32)
    pub machine_type: String,
    /// Current status
    pub status: VmStatus,
    /// Internal IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_ip: Option<String>,
    /// External IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ip: Option<String>,
    /// IPv6 address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<String>,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Associated ICP node mapping (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icp_node: Option<IcpNodeMapping>,
    /// Labels/tags on the VM
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
}

/// Request to provision a new VM
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmProvisionRequest {
    /// Instance name
    pub name: String,
    /// GCP zone
    pub zone: String,
    /// Machine type (e.g., n2-standard-32)
    #[serde(default = "default_machine_type")]
    pub machine_type: String,
    /// Boot disk size in GB
    #[serde(default = "default_disk_size")]
    pub disk_size_gb: u64,
    /// Network tags
    #[serde(default)]
    pub network_tags: Vec<String>,
    /// Labels/tags for the VM
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
    /// Whether to configure as an ICP node
    #[serde(default)]
    pub configure_as_icp_node: bool,
}

fn default_machine_type() -> String {
    "n2-standard-32".to_string()
}

fn default_disk_size() -> u64 {
    500
}

/// Response when listing VMs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmListResponse {
    /// List of VMs
    pub vms: Vec<Vm>,
    /// Total count
    pub total: usize,
}

/// Response when provisioning a VM
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmProvisionResponse {
    /// Operation ID for tracking
    pub operation_id: String,
    /// VM name
    pub vm_name: String,
    /// Zone
    pub zone: String,
    /// Current status
    pub status: String,
}
