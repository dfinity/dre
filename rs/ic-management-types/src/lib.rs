pub mod errors;
pub mod requests;
pub use crate::errors::*;

use ahash::AHashMap;
use candid::{CandidType, Decode};
use core::hash::Hash;
use ic_base_types::NodeId;
use ic_nns_governance::pb::v1::NnsFunction;
use ic_nns_governance::pb::v1::ProposalStatus;
use ic_nns_governance_api::proposal::Action;
use ic_nns_governance_api::ProposalInfo;
use ic_protobuf::registry::node::v1::{IPv4InterfaceConfig, NodeRewardType};
use ic_protobuf::registry::subnet::v1::ChainKeyConfig;
use ic_protobuf::registry::subnet::v1::SubnetFeatures;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use indexmap::IndexMap;
use registry_canister::mutations::do_add_nodes_to_subnet::AddNodesToSubnetPayload;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;
use registry_canister::mutations::do_create_subnet::CreateSubnetPayload;
use registry_canister::mutations::do_deploy_guestos_to_all_subnet_nodes::DeployGuestosToAllSubnetNodesPayload;
use registry_canister::mutations::do_remove_nodes_from_subnet::RemoveNodesFromSubnetPayload;
use registry_canister::mutations::do_revise_elected_replica_versions::ReviseElectedGuestosVersionsPayload;
use registry_canister::mutations::do_update_elected_hostos_versions::UpdateElectedHostosVersionsPayload;
use registry_canister::mutations::do_update_nodes_hostos_version::UpdateNodesHostosVersionPayload;
use registry_canister::mutations::do_update_unassigned_nodes_config::UpdateUnassignedNodesConfigPayload;
use registry_canister::mutations::node_management::do_remove_nodes::RemoveNodesPayload;
use serde::{Deserialize, Serialize};
use std::cmp::{Eq, Ord, PartialEq, PartialOrd};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::net::Ipv6Addr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::OnceLock;
use strum::VariantNames;
use strum_macros::EnumString;
use url::Url;

pub fn filter_map_nns_function_proposals<T: NnsFunctionProposal + candid::CandidType>(proposals: &[ProposalInfo]) -> Vec<(ProposalInfo, T)> {
    proposals
        .iter()
        .filter(|p| ProposalStatus::try_from(p.status).expect("unknown proposal status") != ProposalStatus::Rejected)
        .filter_map(|p| {
            p.proposal
                .as_ref()
                .and_then(|p| p.action.as_ref())
                .ok_or_else(|| anyhow::format_err!("no action"))
                .and_then(|a| match a {
                    Action::ExecuteNnsFunction(function) => {
                        let func = NnsFunction::try_from(function.nns_function)?;
                        Ok((func, function.payload.as_slice()))
                    }
                    _ => Err(anyhow::format_err!("not an NNS function")),
                })
                .and_then(|(function_type, function_payload)| {
                    if function_type == T::TYPE {
                        Decode!(function_payload, T).map_err(|e| anyhow::format_err!("failed decoding candid: {}", e))
                    } else {
                        Err(anyhow::format_err!("unsupported NNS function"))
                    }
                })
                .ok()
                .map(|payload| (p.clone(), payload))
        })
        .collect()
}

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

impl NnsFunctionProposal for UpdateUnassignedNodesConfigPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateUnassignedNodesConfig;
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

impl NnsFunctionProposal for DeployGuestosToAllSubnetNodesPayload {
    const TYPE: NnsFunction = NnsFunction::DeployGuestosToAllSubnetNodes;
}

impl NnsFunctionProposal for UpdateElectedHostosVersionsPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateElectedHostosVersions;
}

impl NnsFunctionProposal for UpdateNodesHostosVersionPayload {
    const TYPE: NnsFunction = NnsFunction::UpdateNodesHostosVersion;
}

impl NnsFunctionProposal for ChangeSubnetMembershipPayload {
    const TYPE: NnsFunction = NnsFunction::ChangeSubnetMembership;
}

impl NnsFunctionProposal for RemoveNodesPayload {
    const TYPE: NnsFunction = NnsFunction::RemoveNodes;
}

impl NnsFunctionProposal for ReviseElectedGuestosVersionsPayload {
    const TYPE: NnsFunction = NnsFunction::ReviseElectedGuestosVersions;
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

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct UpdateNodesHostosVersionsProposal {
    pub proposal_id: u64,
    pub hostos_version_id: String,
    pub node_ids: Vec<NodeId>,
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
    #[serde(default)]
    pub max_ingress_bytes_per_message: u64,
    #[serde(default)]
    pub max_ingress_messages_per_block: u64,
    #[serde(default)]
    pub max_block_payload_size: u64,
    #[serde(default)]
    pub unit_delay_millis: u64,
    #[serde(default)]
    pub initial_notary_delay_millis: u64,
    #[serde(default)]
    pub dkg_interval_length: u64,
    #[serde(default)]
    pub start_as_nns: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub features: Option<SubnetFeatures>,
    #[serde(default)]
    pub max_number_of_canisters: u64,
    #[serde(default)]
    pub ssh_readonly_access: Vec<String>,
    #[serde(default)]
    pub ssh_backup_access: Vec<String>,
    #[serde(default)]
    pub dkg_dealings_per_block: u64,
    #[serde(default)]
    pub is_halted: bool,
    #[serde(default)]
    pub halt_at_cup_height: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub chain_key_config: Option<ChainKeyConfig>,
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

#[derive(Clone, Serialize, Debug, Deserialize, Default)]
pub struct Node {
    pub principal: PrincipalId,
    pub ip_addr: Option<Ipv6Addr>,
    pub operator: Operator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<PrincipalId>,
    pub hostos_release: Option<Release>,
    pub hostos_version: String,
    #[serde(skip)]
    pub cached_features: OnceLock<NodeFeatures>,
    pub dfinity_owned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<TopologyChangeProposal>,
    pub label: Option<String>,
    #[serde(default)]
    pub duplicates: Option<PrincipalId>,
    pub is_api_boundary_node: bool,
    pub chip_id: Option<Vec<u8>>,
    pub public_ipv4_config: Option<IPv4InterfaceConfig>,
    pub node_reward_type: Option<NodeRewardType>,
}

impl Node {
    pub fn id_short(&self) -> String {
        self.principal.to_string().split_once('-').expect("invalid principal").0.to_string()
    }
    pub fn new_test_node(node_number: u64, features: NodeFeatures, dfinity_owned: bool) -> Self {
        Node {
            principal: PrincipalId::new_node_test_id(node_number),
            cached_features: features.into(),
            dfinity_owned: Some(dfinity_owned),
            ..Default::default()
        }
    }
    pub fn with_operator(self, operator: Operator) -> Self {
        Node { operator, ..self }
    }
    pub fn with_subnet_id(self, subnet_id: PrincipalId) -> Self {
        Node {
            subnet_id: Some(subnet_id),
            ..self
        }
    }
    pub fn get_features(&self) -> NodeFeatures {
        let features = if let Some(features) = &self.cached_features.get() {
            // Return a clone of the cached value, if it exists
            (*features).clone()
        } else {
            let country = self
                .operator
                .datacenter
                .as_ref()
                .map(|d| d.country.clone())
                .unwrap_or_else(|| "unknown".to_string());
            let area = self
                .operator
                .datacenter
                .as_ref()
                .map(|d| d.area.clone())
                .unwrap_or_else(|| "unknown".to_string());

            NodeFeatures::from_iter([
                (NodeFeature::Area, area),
                (NodeFeature::Country, country),
                (
                    NodeFeature::Continent,
                    self.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.continent.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (
                    NodeFeature::DataCenterOwner,
                    self.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.owner.name.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (
                    NodeFeature::DataCenter,
                    self.operator
                        .datacenter
                        .as_ref()
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                (NodeFeature::NodeProvider, self.operator.provider.principal.to_string()),
                (NodeFeature::NodeOperator, self.operator.principal.to_string()),
                (NodeFeature::NodeId, self.principal.to_string()),
            ])
        };

        // Cache the calculated value
        self.cached_features.get_or_init(|| features.clone());

        features
    }

    pub fn get_feature(&self, feature: &NodeFeature) -> Option<String> {
        self.get_features().get(feature)
    }

    pub fn matches_feature_value(&self, value: &str) -> bool {
        self.principal.to_string() == *value.to_lowercase()
            || self
                .get_features()
                .feature_map
                .values()
                .any(|v| *v.to_lowercase() == *value.to_lowercase())
    }

    pub fn is_country_from_eu(country: &str) -> bool {
        // (As of 2024) the EU countries are not properly marked in the registry, so we check membership separately.
        let eu_countries: AHashMap<&str, &str> = AHashMap::from_iter([
            ("AT", "Austria"),
            ("BE", "Belgium"),
            ("BG", "Bulgaria"),
            ("CY", "Cyprus"),
            ("CZ", "Czechia"),
            ("DE", "Germany"),
            ("DK", "Denmark"),
            ("EE", "Estonia"),
            ("ES", "Spain"),
            ("FI", "Finland"),
            ("FR", "France"),
            ("GR", "Greece"),
            ("HR", "Croatia"),
            ("HU", "Hungary"),
            ("IE", "Ireland"),
            ("IT", "Italy"),
            ("LT", "Lithuania"),
            ("LU", "Luxembourg"),
            ("LV", "Latvia"),
            ("MT", "Malta"),
            ("NL", "Netherlands"),
            ("PL", "Poland"),
            ("PT", "Portugal"),
            ("RO", "Romania"),
            ("SE", "Sweden"),
            ("SI", "Slovenia"),
            ("SK", "Slovakia"),
        ]);
        eu_countries.contains_key(country)
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node ID: {}\nFeatures:\n{}\nDfinity Owned: {}",
            self.principal,
            self.get_features(),
            self.dfinity_owned.unwrap_or_default()
        )
    }
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.principal.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.principal == other.principal
    }
}

impl Eq for Node {}

#[derive(strum_macros::Display, EnumString, VariantNames, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Debug)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NodeFeature {
    NodeId,
    NodeOperator,
    NodeProvider,
    DataCenter,
    DataCenterOwner,
    Area,    // Represents smaller geographic entities like cities and states
    Country, // Covers larger contexts, like countries or broader regions under shared legal jurisdiction
    Continent,
}

impl NodeFeature {
    pub fn variants() -> Vec<Self> {
        // Generally skip the continent feature as it is not used in the Nakamoto score calculation
        NodeFeature::VARIANTS
            .iter()
            .filter(|f| **f != "continent" && **f != "node_id")
            .map(|f| NodeFeature::from_str(f).unwrap())
            .collect()
    }
    pub fn variants_all() -> Vec<Self> {
        NodeFeature::VARIANTS.iter().map(|f| NodeFeature::from_str(f).unwrap()).collect()
    }
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug, Default)]
pub struct NodeFeatures {
    pub feature_map: IndexMap<NodeFeature, String>,
}

impl std::fmt::Display for NodeFeatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (feature, value) in &self.feature_map {
            writeln!(f, "{}: {}", feature, value)?;
        }
        Ok(())
    }
}

impl NodeFeatures {
    pub fn get(&self, feature: &NodeFeature) -> Option<String> {
        self.feature_map.get(feature).cloned()
    }

    pub fn new_test_feature_set(value: &str) -> Self {
        let mut result = IndexMap::new();
        for feature in NodeFeature::variants() {
            result.insert(feature, value.to_string());
        }
        NodeFeatures { feature_map: result }
    }

    pub fn with_feature_value(&self, feature: &NodeFeature, value: &str) -> Self {
        let mut feature_map = self.feature_map.clone();
        feature_map.insert(feature.clone(), value.to_string());
        NodeFeatures { feature_map }
    }
}

impl FromIterator<(NodeFeature, &'static str)> for NodeFeatures {
    fn from_iter<I: IntoIterator<Item = (NodeFeature, &'static str)>>(iter: I) -> Self {
        Self {
            feature_map: IndexMap::from_iter(iter.into_iter().map(|x| (x.0, String::from(x.1)))),
        }
    }
}

impl FromIterator<(NodeFeature, std::string::String)> for NodeFeatures {
    fn from_iter<I: IntoIterator<Item = (NodeFeature, std::string::String)>>(iter: I) -> Self {
        Self {
            feature_map: IndexMap::from_iter(iter),
        }
    }
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

#[derive(Clone, Serialize, Default, Debug, Deserialize, PartialEq, Eq)]
pub struct Operator {
    pub principal: PrincipalId,
    pub provider: Provider,
    pub node_allowance: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacenter: Option<Datacenter>,
    #[serde(default)]
    pub rewardable_nodes: BTreeMap<String, u32>,
    #[serde(default)]
    pub max_rewardable_nodes: BTreeMap<String, u32>,
    #[serde(default)]
    pub ipv6: String,
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
    pub area: String,
    pub country: String,
    pub continent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

impl PartialEq for Datacenter {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Datacenter {}

#[derive(Clone, Serialize, Default, Debug, Deserialize)]
pub struct DatacenterOwner {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
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
            datacenter: g.physical_system.split('.').nth(1).expect("invalid physical system name").to_string(),
            ipv6: g.ipv6,
            name: g.physical_system.split('.').next().expect("invalid physical system name").to_string(),
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

#[derive(PartialOrd, Ord, Eq, PartialEq, EnumString, Serialize, strum_macros::Display, Deserialize, Debug, Clone, Hash)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Dead,
    Unknown,
}

/// Even if `from_str` is implemented by `EnumString` in derive, public api returns them capitalized and this is the implementation for that convertion
impl HealthStatus {
    pub fn from_str_from_dashboard(alertname: &str, s: &str) -> Self {
        match (alertname, s) {
            (_, "UP" | "UNASSIGNED") => Self::Healthy,
            ("IC_PrometheusTargetMissing", "DEGRADED") => Self::Healthy,
            (_, "DEGRADED") => Self::Degraded,
            (_, "DOWN") => Self::Dead,
            _ => Self::Unknown,
        }
    }
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

#[derive(strum_macros::Display, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Artifact {
    GuestOs,
    HostOs,
}

impl Artifact {
    pub fn s3_folder(&self) -> String {
        match self {
            Artifact::GuestOs => String::from("guest-os"),
            Artifact::HostOs => String::from("host-os"),
        }
    }
    pub fn capitalized(&self) -> String {
        match self {
            Artifact::GuestOs => String::from("Guestos"),
            Artifact::HostOs => String::from("Hostos"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Network {
    pub name: String,
    pub nns_urls: Vec<url::Url>,
}

impl Network {
    pub fn new_unchecked<S: AsRef<str>>(name: S, nns_urls: &[url::Url]) -> anyhow::Result<Self> {
        let (name, nns_urls) = match name.as_ref() {
            "mainnet" => (
                "mainnet".to_string(),
                if nns_urls.is_empty() {
                    vec![Url::from_str("https://ic0.app").unwrap()]
                } else {
                    nns_urls.to_owned()
                },
            ),
            "staging" => (
                "staging".to_string(),
                if nns_urls.is_empty() {
                    [
                        "http://[2600:2c01:21:0:5000:d7ff:fe63:6512]:8080/",
                        "http://[2600:2c01:21:0:5000:beff:fecb:ff53]:8080/",
                        "http://[2600:3000:6100:200:5000:14ff:fecd:3307]:8080/",
                        "http://[2600:3000:6100:200:5000:47ff:fee3:1779]:8080/",
                        "http://[2604:7e00:50:0:5000:a2ff:fed7:e98c]:8080/",
                        "http://[2600:3000:6100:200:5000:b0ff:fe8e:6b7b]:8080/",
                    ]
                    .iter()
                    .map(|s| Url::from_str(s).unwrap())
                    .collect()
                } else {
                    nns_urls.to_owned()
                },
            ),
            _ => (
                name.as_ref().to_string(),
                if nns_urls.is_empty() {
                    return Err(anyhow::anyhow!("No NNS URLs provided"));
                } else {
                    nns_urls.to_owned()
                },
            ),
        };

        Ok(Self { name, nns_urls })
    }

    pub async fn new<S: AsRef<str>>(name: S, nns_urls: &[url::Url]) -> anyhow::Result<Self> {
        let network = Self::new_unchecked(name, nns_urls)?;
        let nns_urls = find_reachable_nns_urls(network.nns_urls).await;
        if nns_urls.is_empty() {
            return Err(anyhow::anyhow!("No reachable NNS URLs provided"));
        }
        Ok(Network { nns_urls, ..network })
    }

    pub fn mainnet_unchecked() -> anyhow::Result<Self> {
        Network::new_unchecked("mainnet", &[])
    }

    pub fn staging_unchecked() -> anyhow::Result<Self> {
        Network::new_unchecked("staging", &[])
    }

    pub fn get_nns_urls(&self) -> &Vec<Url> {
        &self.nns_urls
    }

    pub fn get_nns_urls_string(&self) -> String {
        self.nns_urls.iter().map(|url| url.to_string()).collect::<Vec<String>>().join(",")
    }

    pub fn get_prometheus_endpoint(&self) -> Url {
        match self.name.as_str() {
            "mainnet" => "https://victoria.mainnet.dfinity.network/select/0/prometheus/",
            _ => "https://victoria.testnet.dfinity.network/select/0/prometheus/",
        }
        .parse()
        .expect("Couldn't parse url")
    }

    pub fn legacy_name(&self) -> String {
        match self.name.as_str() {
            "mainnet" => "mercury".to_string(),
            _ => self.name.clone(),
        }
    }

    pub fn is_mainnet(&self) -> bool {
        self.name == "mainnet"
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.get_nns_urls_string())
    }
}

/// Utility function to convert a Url to a host:port string.
fn url_to_host_with_port(url: Url) -> String {
    let host = url.host_str().unwrap_or("");
    let host = if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        // Likely an IPv6 address, enclose in brackets
        format!("[{}]", host)
    } else {
        // IPv4 or hostname
        host.to_string()
    };
    let port = url.port_or_known_default().unwrap_or(8080);

    format!("{}:{}", host, port)
}

/// Utility function to find NNS URLs that the local machine can connect to.
async fn find_reachable_nns_urls(nns_urls: Vec<Url>) -> Vec<Url> {
    // Early return, otherwise `futures::future::select_all` will panic without a good error
    // message.
    if nns_urls.is_empty() {
        return Vec::new();
    }

    let retries_max = 3;
    let timeout_duration = tokio::time::Duration::from_secs(10);

    for i in 1..=retries_max {
        let tasks: Vec<_> = nns_urls
            .iter()
            .map(|url| {
                Box::pin(async move {
                    let host_with_port = url_to_host_with_port(url.clone());

                    match tokio::net::lookup_host(host_with_port.clone()).await {
                        Ok(ips) => {
                            for ip in ips {
                                match tokio::time::timeout(timeout_duration, tokio::net::TcpStream::connect(ip)).await {
                                    Ok(connection) => match connection {
                                        Ok(_) => return Some(url.clone()),
                                        Err(err) => {
                                            eprintln!("WARNING: Failed to connect to {}: {:?}", ip, err);
                                        }
                                    },
                                    Err(err) => {
                                        eprintln!("WARNING: Failed to connect to {}: {:?}", ip, err);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("WARNING: Failed to lookup {}: {:?}", host_with_port, err);
                        }
                    }
                    None
                })
            })
            .collect();

        // Wait for the first task to complete ==> until we have a reachable NNS URL.
        // select_all returns the completed future at position 0, and the remaining futures at position 2.
        let (completed_task, _, remaining_tasks) = futures::future::select_all(tasks).await;
        match completed_task {
            Some(url) => return vec![url],
            None => {
                for task in remaining_tasks {
                    if let Some(url) = task.await {
                        return vec![url];
                    }
                }
                eprintln!(
                    "WARNING: None of the provided NNS urls are reachable. Retrying in 5 seconds... ({}/{})",
                    i, retries_max
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::MockServer;

    #[tokio::test]
    async fn test_network_new_mainnet() {
        let network = Network::new("mainnet", &[]).await.unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &[Url::from_str("https://ic0.app").unwrap()]);
    }

    #[tokio::test]
    async fn test_network_new_mainnet_custom_url() {
        let mock_server = MockServer::start().await;
        let mock_server_url: Url = mock_server.uri().parse().unwrap();
        let network = Network::new("mainnet", &[mock_server_url.clone()]).await.unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &[mock_server_url]);
    }

    #[tokio::test]
    async fn test_network_new_mainnet_custom_and_invalid_url() {
        let mock_server = MockServer::start().await;
        let mock_server_url: Url = mock_server.uri().parse().unwrap();
        let invalid_url1 = Url::from_str("https://unreachable.url1").unwrap();
        let invalid_url2 = Url::from_str("https://unreachable.url2").unwrap();

        let expected_nns_urls = vec![mock_server_url.clone()];

        // Test with the invalid URL last
        let network = Network::new("mainnet", &[mock_server_url.clone(), invalid_url1.clone()]).await.unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &expected_nns_urls);

        // Test with the invalid URL first
        let network = Network::new("mainnet", &[invalid_url1.clone(), mock_server_url.clone()]).await.unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &expected_nns_urls);

        // Test with the valid URL in the middle
        let network = Network::new("mainnet", &[invalid_url1, mock_server_url.clone(), invalid_url2])
            .await
            .unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &expected_nns_urls);
    }

    #[ignore] // Ignore failures since staging IC is not accessible from GitHub actions
    #[tokio::test]
    async fn test_network_new_staging() {
        let network = Network::new("staging", &[]).await.unwrap();

        assert_eq!(network.name, "staging");
        assert_eq!(
            network.get_nns_urls(),
            &vec![Url::from_str("http://[2600:3000:6100:200:5000:b0ff:fe8e:6b7b]:8080").unwrap()]
        );
    }

    #[tokio::test]
    async fn test_network_new_all_unreachable() {
        let name = "custom";
        let nns_urls = &[Url::from_str("https://unreachable.url").unwrap()];
        let network = Network::new(name, nns_urls).await;

        assert!(network.is_err());
        let err = network.err().unwrap().to_string();
        assert_eq!(err, "No reachable NNS URLs provided".to_string())
    }

    #[test]
    fn test_network_get_nns_urls_string() {
        let nns_urls = vec![Url::from_str("https://ic0.app").unwrap(), Url::from_str("https://custom.nns").unwrap()];
        let network = Network {
            name: "mainnet".to_string(),
            nns_urls,
        };

        assert_eq!(network.get_nns_urls_string(), "https://ic0.app/,https://custom.nns/");
    }

    #[test]
    fn test_network_get_prometheus_endpoint() {
        let network = Network {
            name: "mainnet".to_string(),
            nns_urls: vec![],
        };

        assert_eq!(
            network.get_prometheus_endpoint(),
            Url::parse("https://victoria.mainnet.dfinity.network/select/0/prometheus/").unwrap()
        );

        let network = Network {
            name: "some_testnet".to_string(),
            nns_urls: vec![],
        };
        assert_eq!(
            network.get_prometheus_endpoint(),
            Url::parse("https://victoria.testnet.dfinity.network/select/0/prometheus/").unwrap()
        );
    }

    #[test]
    fn test_network_legacy_name() {
        let network = Network {
            name: "mainnet".to_string(),
            nns_urls: vec![],
        };

        assert_eq!(network.legacy_name(), "mercury");
    }
}
