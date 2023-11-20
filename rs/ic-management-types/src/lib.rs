pub mod errors;
pub mod requests;
pub use crate::errors::*;

use candid::{CandidType, Decode};
use core::hash::Hash;
use ic_nns_governance::pb::v1::NnsFunction;
use ic_nns_governance::pb::v1::ProposalInfo;
use ic_nns_governance::pb::v1::ProposalStatus;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use registry_canister::mutations::do_add_nodes_to_subnet::AddNodesToSubnetPayload;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use registry_canister::mutations::do_create_subnet::CreateSubnetPayload;
use registry_canister::mutations::do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload;
use registry_canister::mutations::do_update_elected_hostos_versions::UpdateElectedHostosVersionsPayload;
use registry_canister::mutations::do_update_elected_replica_versions::UpdateElectedReplicaVersionsPayload;
use registry_canister::mutations::do_update_subnet_replica::UpdateSubnetReplicaVersionPayload;
use registry_canister::mutations::node_management::do_remove_nodes::RemoveNodesPayload;
use serde::{Deserialize, Serialize};
use std::cmp::{Eq, Ord, PartialEq, PartialOrd};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::net::Ipv6Addr;
use std::ops::Deref;
use std::str::FromStr;
use strum::VariantNames;
use strum_macros::{Display, EnumString, EnumVariantNames};
use url::Url;

pub trait NnsFunctionProposal: CandidType + serde::de::DeserializeOwned {
    const TYPE: NnsFunction;
    fn decode(function_type: NnsFunction, function_payload: &[u8]) -> anyhow::Result<Self> {
        if function_type == Self::TYPE {
            Decode!(function_payload, Self).map_err(|e| anyhow::format_err!("failed decoding candid: {}", e))
        } else {
            Err(anyhow::format_err!("unsupported NNS function"))
        }
    }
}

impl NnsFunctionProposal for AddNodesToSubnetPayload {
    const TYPE: NnsFunction = NnsFunction::AddNodeToSubnet;
}

impl NnsFunctionProposal for RemoveNodesFromSubnetPayload {
    const TYPE: NnsFunction = NnsFunction::RemoveNodesFromSubnet;
}

impl NnsFunctionProposal for CreateSubnetPayload {
    const TYPE: NnsFunction = NnsFunction::CreateSubnet;
}

impl NnsFunctionProposal for UpdateSubnetReplicaVersionPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateSubnetReplicaVersion;
}

impl NnsFunctionProposal for UpdateElectedHostosVersionsPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateNodesHostosVersion;
}

impl NnsFunctionProposal for ChangeSubnetMembershipPayload {
    const TYPE: NnsFunction = NnsFunction::ChangeSubnetMembership;
}

impl NnsFunctionProposal for RemoveNodesPayload {
    const TYPE: NnsFunction = NnsFunction::RemoveNodes;
}

impl NnsFunctionProposal for UpdateElectedReplicaVersionsPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateElectedReplicaVersions;
}

pub trait TopologyChangePayload: NnsFunctionProposal {
    fn get_added_node_ids(&self) -> Vec<PrincipalId>;
    fn get_removed_node_ids(&self) -> Vec<PrincipalId>;
    fn get_subnet(&self) -> Option<PrincipalId>;
}

impl TopologyChangePayload for CreateSubnetPayload {
    fn get_added_node_ids(&self) -> Vec<PrincipalId> {
        self.node_ids.iter().map(|node_id| node_id.get()).collect()
    }

    fn get_removed_node_ids(&self) -> Vec<PrincipalId> {
        vec![]
    }

    fn get_subnet(&self) -> Option<PrincipalId> {
        None
    }
}

impl TopologyChangePayload for AddNodesToSubnetPayload {
    fn get_added_node_ids(&self) -> Vec<PrincipalId> {
        self.node_ids.iter().map(|node_id| node_id.get()).collect()
    }

    fn get_removed_node_ids(&self) -> Vec<PrincipalId> {
        vec![]
    }

    fn get_subnet(&self) -> Option<PrincipalId> {
        Some(self.subnet_id)
    }
}

impl TopologyChangePayload for RemoveNodesFromSubnetPayload {
    fn get_added_node_ids(&self) -> Vec<PrincipalId> {
        vec![]
    }

    fn get_removed_node_ids(&self) -> Vec<PrincipalId> {
        self.node_ids.iter().map(|node_id| node_id.get()).collect()
    }

    fn get_subnet(&self) -> Option<PrincipalId> {
        None
    }
}

impl TopologyChangePayload for ChangeSubnetMembershipPayload {
    fn get_added_node_ids(&self) -> Vec<PrincipalId> {
        self.node_ids_add.iter().map(|node_id| node_id.get()).collect()
    }

    fn get_removed_node_ids(&self) -> Vec<PrincipalId> {
        self.node_ids_remove.iter().map(|node_id| node_id.get()).collect()
    }

    fn get_subnet(&self) -> Option<PrincipalId> {
        Some(self.subnet_id)
    }
}

impl TopologyChangePayload for RemoveNodesPayload {
    fn get_added_node_ids(&self) -> Vec<PrincipalId> {
        vec![]
    }

    fn get_removed_node_ids(&self) -> Vec<PrincipalId> {
        self.node_ids.iter().map(|node_id| node_id.get()).collect()
    }

    fn get_subnet(&self) -> Option<PrincipalId> {
        None
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TopologyChangeProposal {
    pub node_ids_added: Vec<PrincipalId>,
    pub node_ids_removed: Vec<PrincipalId>,
    pub subnet_id: Option<PrincipalId>,
    pub id: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct UpdateElectedReplicaVersionsProposal {
    pub proposal_id: u64,
    pub version_elect: String,
    pub versions_unelect: Vec<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct UpdateElectedHostosVersionsProposal {
    pub proposal_id: u64,
    pub version_elect: String,
    pub versions_unelect: Vec<String>,
}

impl<T: TopologyChangePayload> From<(ProposalInfo, T)> for TopologyChangeProposal {
    fn from((info, payload): (ProposalInfo, T)) -> Self {
        Self {
            subnet_id: payload.get_subnet(),
            node_ids_added: payload.get_added_node_ids(),
            node_ids_removed: payload.get_removed_node_ids(),
            id: info.id.unwrap().id,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Subnet {
    pub principal: PrincipalId,
    pub nodes: Vec<Node>,
    pub subnet_type: SubnetType,
    pub metadata: SubnetMetadata,
    pub replica_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<TopologyChangeProposal>,
    pub replica_release: Option<Release>,
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

#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct Node {
    pub principal: PrincipalId,
    pub ip_addr: Ipv6Addr,
    pub operator: Operator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<PrincipalId>,
    pub hostos_release: Option<Release>,
    pub hostos_version: String,
    pub dfinity_owned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<TopologyChangeProposal>,
    pub label: Option<String>,
    #[serde(default)]
    pub decentralized: bool,
    pub duplicates: Option<PrincipalId>,
}

#[derive(
    Display, EnumString, EnumVariantNames, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Debug,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NodeFeature {
    NodeProvider,
    DataCenter,
    DataCenterOwner,
    City,
    Country,
    Continent,
}

impl NodeFeature {
    pub fn variants() -> Vec<Self> {
        NodeFeature::VARIANTS
            .iter()
            .map(|f| NodeFeature::from_str(f).unwrap())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MinNakamotoCoefficients {
    pub coefficients: BTreeMap<NodeFeature, f64>,
    pub average: f64,
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

#[derive(Clone, Serialize, Default, Debug, Deserialize)]
pub struct Operator {
    pub principal: PrincipalId,
    pub provider: Provider,
    pub allowance: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacenter: Option<Datacenter>,
}

#[derive(Clone, Serialize, Default, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Provider {
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
    pub decentralized: bool,
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
            decentralized: g.ipv6.segments()[4] == 0x6801,
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

#[derive(PartialOrd, Ord, Eq, PartialEq, EnumString, Serialize, Display, Deserialize, Debug, Clone)]
pub enum Status {
    Healthy,
    Degraded,
    Dead,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Release {
    pub commit_hash: String,
    pub branch: String,
    pub name: String,
    pub time: chrono::NaiveDateTime,
    pub previous_patch_release: Option<Box<Release>>,
}

impl Release {
    pub fn patch_count(&self) -> u32 {
        match &self.previous_patch_release {
            Some(rv) => rv.patch_count(),
            None => 0,
        }
    }

    pub fn patches(&self, replica_release: &Release) -> bool {
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

    pub fn patches_for(&self, commit_hash: &str) -> Result<Vec<Release>, String> {
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

    pub fn get(&self, commit_hash: &str) -> Result<Release, String> {
        if self.commit_hash == *commit_hash {
            Ok(self.clone())
        } else if let Some(previous) = &self.previous_patch_release {
            previous.get(commit_hash)
        } else {
            Err("doesn't patch this release".to_string())
        }
    }
}

pub struct ArtifactReleases {
    pub artifact: Artifact,
    pub releases: Vec<Release>,
}

impl ArtifactReleases {
    pub fn new(artifact: Artifact) -> ArtifactReleases {
        ArtifactReleases {
            artifact,
            releases: Vec::new(),
        }
    }
}

#[derive(Display, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Artifact {
    Replica,
    HostOs,
}

impl Artifact {
    pub fn s3_folder(&self) -> String {
        match self {
            Artifact::Replica => String::from("guest-os"),
            Artifact::HostOs => String::from("host-os"),
        }
    }
    pub fn capitalized(&self) -> String {
        match self {
            Artifact::Replica => String::from("Replica"),
            Artifact::HostOs => String::from("Hostos"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, strum_macros::Display)]
#[strum(serialize_all = "lowercase")]
pub enum Network {
    Staging,
    Mainnet,
    Url(url::Url),
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "mainnet" => Self::Mainnet,
            "staging" => Self::Staging,
            _ => Self::Url(url::Url::from_str(s).map_err(|e| format!("{}", e))?),
        })
    }
}

impl Network {
    pub fn get_url(&self) -> Url {
        match self {
            Network::Mainnet => Url::from_str("https://ic0.app").unwrap(),
            // Workaround for staging boundary node not working properly (503 Service unavailable)
            Network::Staging => Url::from_str("https://[2600:3004:1200:1200:5000:62ff:fedc:fe3c]:8080").unwrap(),
            Self::Url(url) => url.clone(),
        }
    }

    pub fn legacy_name(&self) -> String {
        match self {
            Network::Mainnet => "mercury".to_string(),
            Network::Staging => "staging".to_string(),
            Self::Url(url) => format!("testnet-{url}"),
        }
    }
}
