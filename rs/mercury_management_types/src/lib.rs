pub mod requests;

use ic_base_types::RegistryVersion;
use ic_nns_governance::pb::v1::ProposalStatus;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::cmp::{Eq, Ord, PartialEq, PartialOrd};
use std::convert::TryFrom;
use std::net::Ipv6Addr;
use std::ops::Deref;
use std::sync::Arc;
use strum_macros::{Display, EnumString};

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Subnet {
    #[serde_as(as = "DisplayFromStr")]
    pub principal: PrincipalId,
    pub nodes: Vec<Node>,
    pub subnet_type: SubnetType,
    pub metadata: SubnetMetadata,
    pub replica_version: String,
    pub version: RegistryVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<TopologyProposal>,
}

type Application = String;
type Label = String;

#[derive(Clone, Serialize, Default, Deserialize)]
pub struct SubnetMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applications: Option<Vec<Application>>,
}

#[serde_as]
#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct Node {
    #[serde_as(as = "DisplayFromStr")]
    pub principal: PrincipalId,
    pub ip_addr: Ipv6Addr,
    pub operator: Operator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet: Option<PrincipalId>,
    pub labels: Vec<NodeLabel>,
    pub version: RegistryVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<TopologyProposal>,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct TopologyProposal {
    pub id: u64,
    pub kind: TopologyProposalKind,
    pub status: TopologyProposalStatus,
}

#[derive(EnumString, Clone, Deserialize, Debug, Serialize)]
pub enum TopologyProposalStatus {
    Open,
    Executed,
}

impl TryFrom<ProposalStatus> for TopologyProposalStatus {
    type Error = String;

    fn try_from(value: ProposalStatus) -> Result<Self, Self::Error> {
        match value {
            ProposalStatus::Open => Ok(Self::Open),
            ProposalStatus::Executed => Ok(Self::Executed),
            _ => Err("cannot convert to topology proposal".to_string()),
        }
    }
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyProposalKind {
    ReplaceNode(ReplaceNodeProposalInfo),
    CreateSubnet(CreateSubnetProposalInfo),
}

#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct ReplaceNodeProposalInfo {
    pub old_nodes: Vec<PrincipalId>,
    pub new_nodes: Vec<PrincipalId>,
    pub first: bool,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct CreateSubnetProposalInfo {
    pub nodes: Vec<PrincipalId>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeLabel {
    pub name: NodeLabelName,
}

impl Serialize for NodeLabel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Ok(serde_json::json!({
            "name": self.name.to_string(),
            "value": self.name.description(),
        })
        .serialize(serializer)
        .unwrap())
    }
}

#[derive(Display, EnumString, Clone, Debug, PartialEq, Eq, Deserialize, Hash)]
pub enum NodeLabelName {
    #[strum(to_string = "DFINITY")]
    #[serde(alias = "DFINITY", rename(serialize = "DFINITY"))]
    DFINITYOwned,
    #[strum(to_string = "NNS ready")]
    #[serde(alias = "NNS ready", rename(serialize = "NNS ready"))]
    NNSReady,
    #[strum(to_string = "Old CUP")]
    #[serde(alias = "Old CUP", rename(serialize = "Old CUP"))]
    OldCUP,
}

impl NodeLabelName {
    pub fn name(&self) -> String {
        self.to_string()
    }
    pub fn description(&self) -> String {
        match self {
            NodeLabelName::DFINITYOwned => "Owned by DFINITY",
            NodeLabelName::NNSReady => "Provisioned for participating exclusively in the NNS subnet",
            NodeLabelName::OldCUP => {
                "CUP creation in this version of nodemanager running is incompatible with new versions"
            }
        }
        .into()
    }
}

#[serde_as]
#[derive(Clone, Serialize, Default, Debug, Deserialize)]
pub struct Operator {
    #[serde_as(as = "DisplayFromStr")]
    pub principal: PrincipalId,
    pub provider: Provider,
    pub allowance: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacenter: Option<Datacenter>,
}

#[serde_as]
#[derive(Clone, Serialize, Default, Debug, Deserialize)]
pub struct Provider {
    #[serde_as(as = "DisplayFromStr")]
    pub principal: PrincipalId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub website: Option<String>,
}

#[derive(Clone, Serialize, Default, Debug, Deserialize)]
pub struct Datacenter {
    pub name: String,
    pub owner: DatacenterOwner,
    pub city: String,
    pub country: String,
    pub continent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

#[derive(Clone, Serialize, Default, Debug, Deserialize)]
pub struct DatacenterOwner {
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct Host {
    pub datacenter: String,
    pub ipv6: Ipv6Addr,
    pub name: String,
    pub system_serial: String,
    pub labels: Option<Vec<NodeLabel>>,
}

// https://ic-api.internetcomputer.org/api/v2/locations
#[derive(Clone, Serialize, Deserialize)]
pub struct Location {
    pub key: String,
    pub latitude: f64,
    pub longitude: f64,
    pub name: String,
    pub node_operator: String,
}

// https://ic-api.internetcomputer.org/api/node-providers/list
#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderDetails {
    pub display_name: String,
    pub principal_id: PrincipalId,
    pub website: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeReplacements {
    pub removed: Vec<PrincipalId>,
    pub added: Vec<PrincipalId>,
}

#[derive(PartialOrd, Ord, Eq, PartialEq, Clone, Serialize, Deserialize, Debug, EnumString)]
pub enum Health {
    Offline,
    Degraded,
    Healthy,
    Unknown,
}

impl From<i64> for Health {
    fn from(value: i64) -> Self {
        match value {
            1 => Self::Healthy,
            0 => Self::Offline,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatusSource {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub health: Health,
    pub ip_addr: Ipv6Addr,
    pub sources: Vec<NodeStatusSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersion {
    pub principal: PrincipalId,
    pub replica_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplicaRelease {
    pub commit_hash: String,
    pub branch: String,
    pub name: String,
    pub time: chrono::NaiveDateTime,
    pub previous_patch_release: Option<Arc<ReplicaRelease>>,
}

impl ReplicaRelease {
    pub fn patch_count(&self) -> u32 {
        match &self.previous_patch_release {
            Some(rv) => rv.patch_count(),
            None => 0,
        }
    }

    pub fn patches(&self, replica_release: &ReplicaRelease) -> bool {
        match &self.previous_patch_release {
            Some(rv) => rv.deref().eq(replica_release) || rv.patches(replica_release),
            None => false,
        }
    }
}
