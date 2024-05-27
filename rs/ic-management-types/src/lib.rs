pub mod errors;
pub mod requests;
pub use crate::errors::*;

use candid::{CandidType, Decode};
use core::hash::Hash;
use ic_base_types::NodeId;
use ic_nns_governance::pb::v1::NnsFunction;
use ic_nns_governance::pb::v1::ProposalInfo;
use ic_nns_governance::pb::v1::ProposalStatus;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
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
use strum::VariantNames;
use strum_macros::EnumString;
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

#[derive(strum_macros::Display, EnumString, VariantNames, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Debug)]
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
        NodeFeature::VARIANTS.iter().map(|f| NodeFeature::from_str(f).unwrap()).collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
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
            datacenter: g.physical_system.split('.').nth(1).expect("invalid physical system name").to_string(),
            ipv6: g.ipv6,
            name: g.physical_system.split('.').next().expect("invalid physical system name").to_string(),
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

#[derive(PartialOrd, Ord, Eq, PartialEq, EnumString, Serialize, strum_macros::Display, Deserialize, Debug, Clone, Hash)]
pub enum Status {
    Healthy,
    Degraded,
    Dead,
    Unknown,
}

/// Even if `from_str` is implemented by `EnumString` in derive, public api returns them capitalized and this is the implementation for that convertion
impl Status {
    pub fn from_str_from_dashboard(s: &str) -> Self {
        match s {
            "UP" | "UNASSIGNED" => Self::Healthy,
            "DEGRADED" => Self::Degraded,
            "DOWN" => Self::Dead,
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

#[derive(strum_macros::Display, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
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
    pub async fn new<S: AsRef<str>>(name: S, nns_urls: &Vec<url::Url>) -> Result<Self, String> {
        let (name, nns_urls) = match name.as_ref() {
            "mainnet" => (
                "mainnet".to_string(),
                if nns_urls.is_empty() {
                    vec![Url::from_str("https://ic0.app").unwrap()]
                } else {
                    nns_urls.clone()
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
                    nns_urls.clone()
                },
            ),
            _ => (
                name.as_ref().to_string(),
                if nns_urls.is_empty() {
                    return Err("No NNS URLs provided".to_string());
                } else {
                    nns_urls.clone()
                },
            ),
        };
        let nns_urls = find_reachable_nns_urls(nns_urls).await;
        if nns_urls.is_empty() {
            return Err("No reachable NNS URLs provided".to_string());
        }
        Ok(Network { name, nns_urls })
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
        let network = Network::new("mainnet", &vec![]).await.unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &vec![Url::from_str("https://ic0.app").unwrap()]);
    }

    #[tokio::test]
    async fn test_network_new_mainnet_custom_url() {
        let mock_server = MockServer::start().await;
        let mock_server_url: Url = mock_server.uri().parse().unwrap();
        let network = Network::new("mainnet", &vec![mock_server_url.clone()]).await.unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &vec![mock_server_url]);
    }

    #[tokio::test]
    async fn test_network_new_mainnet_custom_and_invalid_url() {
        let mock_server = MockServer::start().await;
        let mock_server_url: Url = mock_server.uri().parse().unwrap();
        let invalid_url1 = Url::from_str("https://unreachable.url1").unwrap();
        let invalid_url2 = Url::from_str("https://unreachable.url2").unwrap();

        let expected_nns_urls = vec![mock_server_url.clone()];

        // Test with the invalid URL last
        let network = Network::new("mainnet", &vec![mock_server_url.clone(), invalid_url1.clone()])
            .await
            .unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &expected_nns_urls);

        // Test with the invalid URL first
        let network = Network::new("mainnet", &vec![invalid_url1.clone(), mock_server_url.clone()])
            .await
            .unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &expected_nns_urls);

        // Test with the valid URL in the middle
        let network = Network::new("mainnet", &vec![invalid_url1, mock_server_url.clone(), invalid_url2])
            .await
            .unwrap();

        assert_eq!(network.name, "mainnet");
        assert_eq!(network.get_nns_urls(), &expected_nns_urls);
    }

    #[ignore] // Ignore failures since staging IC is not accessible from GitHub actions
    #[tokio::test]
    async fn test_network_new_staging() {
        let network = Network::new("staging", &vec![]).await.unwrap();

        assert_eq!(network.name, "staging");
        assert_eq!(
            network.get_nns_urls(),
            &vec![Url::from_str("http://[2600:3000:6100:200:5000:b0ff:fe8e:6b7b]:8080").unwrap()]
        );
    }

    #[tokio::test]
    async fn test_network_new_all_unreachable() {
        let name = "custom";
        let nns_urls = vec![Url::from_str("https://unreachable.url").unwrap()];
        let network = Network::new(name, &nns_urls).await;

        assert_eq!(network, Err("No reachable NNS URLs provided".to_string()));
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
