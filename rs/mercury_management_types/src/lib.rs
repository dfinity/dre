pub mod requests;

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
use strum_macros::EnumString;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Subnet {
    #[serde_as(as = "DisplayFromStr")]
    pub principal: PrincipalId,
    pub nodes: Vec<Node>,
    pub subnet_type: SubnetType,
    pub metadata: SubnetMetadata,
    pub replica_version: String,
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
    pub dfinity_owned: bool,
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
pub struct Guest {
    pub datacenter: String,
    pub ipv6: Ipv6Addr,
    pub name: String,
    pub dfinity_owned: bool,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct FactsDBGuest {
    pub name: String,
    pub node_type: String,
    pub ipv6: Ipv6Addr,
    pub principal: String,
    pub subnet: String,
    pub physical_system: String,
}

impl From<FactsDBGuest> for Guest {
    fn from(g: FactsDBGuest) -> Self {
        Guest {
            datacenter: g
                .physical_system
                .split('.')
                .nth(1)
                .expect("invalid physical system name")
                .to_string(),
            ipv6: g.ipv6,
            name: g
                .physical_system
                .split('.')
                .next()
                .expect("invalid physical system name")
                .to_string(),
            dfinity_owned: g.node_type.contains("dfinity"),
        }
    }
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

// https://ic-api.internetcomputer.org/api/v3/node-providers
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeProvidersResponse {
    pub node_providers: Vec<NodeProviderDetails>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeProviderDetails {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

    pub fn contains_patch(&self, commit_hash: &str) -> bool {
        self.commit_hash == commit_hash
            || self
                .previous_patch_release
                .as_ref()
                .map(|r| r.contains_patch(commit_hash))
                .unwrap_or_default()
    }

    pub fn patches_for(&self, commit_hash: &str) -> Result<Vec<ReplicaRelease>, String> {
        if self.commit_hash == *commit_hash {
            Ok(vec![])
        } else if let Some(previous) = &self.previous_patch_release {
            previous.patches_for(commit_hash).map(|mut patches| {
                patches.push(self.clone());
                patches
            })
        } else {
            Err("doesn't patch this release".to_string())
        }
    }

    pub fn get(&self, commit_hash: &str) -> Result<ReplicaRelease, String> {
        if self.commit_hash == *commit_hash {
            Ok(self.clone())
        } else if let Some(previous) = &self.previous_patch_release {
            previous.get(commit_hash)
        } else {
            Err("doesn't patch this release".to_string())
        }
    }
}

#[derive(EnumString, Clone)]
#[strum(serialize_all = "snake_case")]
pub enum Network {
    Staging,
    Mainnet,
}
