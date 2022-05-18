use crate::proposal;
use async_trait::async_trait;
use decentralization::network::{AvailableNodesQuerier, NetworkError, SubnetQuerier};
use ic_base_types::RegistryVersion;
use ic_interfaces::registry::RegistryValue;
use ic_interfaces::registry::RegistryVersionedRecord;
use ic_registry_keys::{NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::info;
use mercury_management_types::{
    Datacenter, DatacenterOwner, Host, Location, Node, NodeLabel, NodeLabelName, Operator, Provider, ProviderDetails,
    ReplicaRelease, Subnet, SubnetMetadata, TopologyProposalKind, TopologyProposalStatus,
};
use std::convert::TryFrom;
use std::{
    collections::{HashMap, HashSet},
    net::Ipv6Addr,
};

use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord,
};

use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_registry_keys::{make_blessed_replica_version_key, DATA_CENTER_KEY_PREFIX};

use serde::Deserialize;
use std::str::FromStr;

use crate::gitlab::{CommitRef, CommitRefs};
use gitlab::api::AsyncQuery;

use lazy_static::lazy_static;
use regex::Regex;

use anyhow::Result;

#[derive(Clone)]
pub struct RegistryState {
    subnet_metadata: MetadataRegistry,
    version: u64,
    subnet_records: HashMap<PrincipalId, (RegistryVersion, SubnetRecord)>,
    node_records: HashMap<PrincipalId, (RegistryVersion, NodeRecord)>,
    operator_records: HashMap<PrincipalId, NodeOperatorRecord>,
    data_center_records: HashMap<String, DataCenterRecord>,
    subnets: HashMap<PrincipalId, Subnet>,
    nodes: HashMap<PrincipalId, Node>,
    operators: HashMap<PrincipalId, Operator>,
    hosts: Vec<Host>,

    removed_node_records: HashMap<PrincipalId, (RegistryVersion, NodeRecord)>,
    removed_nodes: HashMap<PrincipalId, Node>,

    blessed_versions: Vec<String>,
    replica_releases: Vec<ReplicaRelease>,
    gitlab_client_archived: Option<gitlab::AsyncGitlab>,
    gitlab_client_public: Option<gitlab::AsyncGitlab>,
}

impl RegistryState {
    pub(crate) fn new(
        gitlab_client_archived: Option<gitlab::AsyncGitlab>,
        gitlab_client_public: Option<gitlab::AsyncGitlab>,
    ) -> Self {
        let labels =
            serde_yaml::from_str::<Vec<Label>>(include_str!("../data/labels.yaml")).expect("invalid configuration");

        Self {
            version: 0,
            subnet_metadata: MetadataRegistry::new(),
            subnet_records: HashMap::new(),
            node_records: HashMap::new(),
            operator_records: HashMap::new(),
            data_center_records: HashMap::new(),
            subnets: HashMap::<PrincipalId, Subnet>::new(),
            nodes: HashMap::new(),
            operators: HashMap::new(),
            hosts: serde_json::from_str::<Vec<Host>>(include_str!("../data/hosts.json"))
                .unwrap()
                .into_iter()
                .map(|h| Host {
                    labels: labels
                        .iter()
                        .filter(|l| l.hosts.contains(&h.name))
                        .map(|l| NodeLabel {
                            name: NodeLabelName::from_str(l.name.as_str()).unwrap(),
                        })
                        .collect::<Vec<_>>()
                        .into(),
                    ..h
                })
                .collect(),
            removed_nodes: HashMap::new(),
            removed_node_records: HashMap::new(),
            blessed_versions: Vec::new(),
            replica_releases: Vec::new(),
            gitlab_client_archived,
            gitlab_client_public,
        }
    }

    pub(crate) async fn update(
        &mut self,
        deltas: Vec<RegistryVersionedRecord<Vec<u8>>>,
        locations: Vec<Location>,
        providers: Vec<ProviderDetails>,
    ) -> anyhow::Result<()> {
        let mut latest_version = 0;

        let locations = locations
            .into_iter()
            .map(|l| (l.key.clone(), l))
            .collect::<HashMap<_, _>>();
        let providers = providers
            .into_iter()
            .map(|p| (p.principal_id, p))
            .collect::<HashMap<_, _>>();

        for versioned_record in deltas.into_iter() {
            latest_version = versioned_record.version.get();
            if let Ok(principal_record) = PrincipalRecord::try_from(versioned_record.clone()) {
                match principal_record.value {
                    PrincipalRecordValue::Subnet(Some(subnet_record)) => {
                        self.subnet_metadata.add(
                            principal_record.principal,
                            SubnetType::try_from(subnet_record.subnet_type).unwrap(),
                        );
                        self.subnet_records
                            .insert(principal_record.principal, (principal_record.version, subnet_record));
                    }
                    PrincipalRecordValue::Subnet(None) => {
                        self.subnet_metadata.remove(principal_record.principal);
                        self.subnet_records.remove(&principal_record.principal);
                    }
                    PrincipalRecordValue::Node(Some(node_record)) => {
                        self.node_records
                            .insert(principal_record.principal, (principal_record.version, node_record));
                    }
                    PrincipalRecordValue::Node(None) => {
                        if let Some((_, nr)) = self.node_records.remove(&principal_record.principal) {
                            self.removed_node_records
                                .insert(principal_record.principal, (principal_record.version, nr));
                        }
                    }
                    PrincipalRecordValue::Operator(Some(operator)) => {
                        self.operator_records.insert(principal_record.principal, operator);
                    }
                    PrincipalRecordValue::Operator(None) => {
                        // self.operator_records.remove(&principal_record.
                        // principal);
                    }
                }
            } else if let Some(dc) = versioned_record.key.strip_prefix(DATA_CENTER_KEY_PREFIX) {
                if let Some(data) = versioned_record.value {
                    self.data_center_records
                        .insert(dc.to_string(), DataCenterRecord::decode(data.as_slice())?);
                } else {
                    self.data_center_records.remove(dc);
                }
            } else if versioned_record.key == make_blessed_replica_version_key() {
                info!("Updating blessed versions");

                self.blessed_versions = BlessedReplicaVersions::decode(
                    versioned_record
                        .value
                        .expect("blessed versions value missing")
                        .as_slice(),
                )
                .expect("failed to decode blessed replica versions")
                .blessed_version_ids;
            }
        }

        self.update_replica_releases().await?;
        self.update_operators(locations, providers);
        self.update_nodes();
        self.update_removed_nodes();
        self.update_subnets();

        self.version = latest_version;

        Ok(())
    }

    async fn update_replica_releases(&mut self) -> Result<()> {
        const STARTING_VERSION: &str = "e86ac9553a8eddbeffaa29267a216c9554d3a0c6";
        let blessed_versions_diff = self
            .blessed_versions
            .iter()
            .skip_while(|v| *v != STARTING_VERSION)
            .filter(|v| !self.replica_releases.iter().any(|rr| rr.commit_hash == **v))
            .collect::<Vec<_>>();

        if let (Some(gitlab_client_archived), Some(gitlab_client_public)) =
            (&self.gitlab_client_archived, &self.gitlab_client_public)
        {
            for version in blessed_versions_diff {
                let endpoint_archived = CommitRefs::builder()
                    .project("dfinity-lab/core/ic")
                    .commit(version)
                    .build()
                    .expect("unable to build refs query");
                let endpoint_public = CommitRefs::builder()
                    .project("dfinity-lab/public/ic")
                    .commit(version)
                    .build()
                    .expect("unable to build refs query");

                let results: Vec<Result<Vec<CommitRef>, _>> = futures::future::join_all(vec![
                    gitlab::api::paged(endpoint_archived, gitlab::api::Pagination::All)
                        .query_async(gitlab_client_archived),
                    gitlab::api::paged(endpoint_public, gitlab::api::Pagination::All).query_async(gitlab_client_public),
                ])
                .await;

                let refs_result = results.iter().find(|r| r.is_ok()).map(|r| r.as_ref().unwrap());

                match refs_result {
                    Some(refs) => {
                        lazy_static! {
                            static ref RELEASE_BRANCH_GROUP: &'static str = "release_branch";
                            static ref RELEASE_NAME_GROUP: &'static str = "release_name";
                            static ref DATETIME_NAME_GROUP: &'static str = "datetime";
                            // example: rc--2021-09-13_18-32
                            static ref RE: Regex = Regex::new(&format!(r#"^(?P<{}>(?P<{}>rc--(?P<{}>\d{{4}}-\d{{2}}-\d{{2}}_\d{{2}}-\d{{2}}))(?P<discardable_suffix>.*))$"#,
                                *RELEASE_BRANCH_GROUP,
                                *RELEASE_NAME_GROUP,
                                *DATETIME_NAME_GROUP,
                            )).unwrap();
                        }
                        if let Some(captures) = refs.iter().find_map(|r| match r.kind.as_str() {
                            "branch" => RE.captures(&r.name),
                            _ => None,
                        }) {
                            let release_name = captures
                                .name(&RELEASE_NAME_GROUP)
                                .expect("release regex misconfiguration")
                                .as_str();
                            let release_branch = captures
                                .name(&RELEASE_BRANCH_GROUP)
                                .expect("release regex misconfiguration")
                                .as_str();
                            let rr = ReplicaRelease {
                                name: release_name.to_string(),
                                branch: release_branch.to_string(),
                                commit_hash: version.clone(),
                                previous_patch_release: self
                                    .replica_releases
                                    .iter()
                                    .rfind(|rr| rr.name == release_name)
                                    .map(|rr| rr.clone().into()),
                                time: chrono::NaiveDateTime::parse_from_str(
                                    captures
                                        .name(&DATETIME_NAME_GROUP)
                                        .expect("release regex misconfiguration")
                                        .as_str(),
                                    "%Y-%m-%d_%H-%M",
                                )
                                .expect("invalid datetime format"),
                            };
                            self.replica_releases.push(rr);
                        }
                    }
                    None => {
                        if results.iter().all(|r| {
                            if let gitlab::api::ApiError::Gitlab { msg } =
                                r.as_ref().expect_err("all results should be errors")
                            {
                                msg.contains(reqwest::StatusCode::NOT_FOUND.as_str())
                            } else {
                                false
                            }
                        }) {
                            return Err(anyhow::format_err!("no releases found for version {}", version));
                        } else {
                            return Err(anyhow::format_err!(
                                "gitlab ref queries failed: {}",
                                results
                                    .iter()
                                    .map(|r| r.as_ref().unwrap_err().to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ));
                        }
                    }
                }
            }
        }

        self.replica_releases.sort_by(|rr1, rr2| match rr1.time.cmp(&rr2.time) {
            std::cmp::Ordering::Equal => rr1.patch_count().cmp(&rr2.patch_count()),
            other => other,
        });

        Ok(())
    }

    fn update_operators(
        &mut self,
        locations: HashMap<String, Location>,
        providers: HashMap<PrincipalId, ProviderDetails>,
    ) {
        self.operators = self
            .operator_records
            .iter()
            .map(|(p, or)| {
                (
                    *p,
                    Operator {
                        principal: *p,
                        provider: PrincipalId::try_from(&or.node_provider_principal_id[..])
                            .map(|p| Provider {
                                name: providers.get(&p).map(|pd| pd.display_name.clone()),
                                website: providers.get(&p).and_then(|pd| pd.website.clone()),
                                principal: p,
                            })
                            .expect("provider missing from operator record"),
                        allowance: or.node_allowance,
                        datacenter: self.data_center_records.get(&or.dc_id).map(|dc| {
                            let (continent, country, city): (_, _, _) = dc
                                .region
                                .splitn(3, ',')
                                .map(|s| s.to_string())
                                .collect_tuple()
                                .unwrap_or(("Unknown".to_string(), "Unknown".to_string(), "Unknown".to_string()));

                            Datacenter {
                                name: dc.id.clone(),
                                city,
                                country,
                                continent,
                                owner: DatacenterOwner {
                                    name: locations
                                        .get(&dc.id)
                                        .map(|l| l.node_operator.clone())
                                        .unwrap_or_else(|| "Unknown".to_string()),
                                },
                                latitude: dc.gps.clone().map(|l| l.latitude as f64),
                                longitude: dc.gps.clone().map(|l| l.longitude as f64),
                            }
                        }),
                    },
                )
            })
            .collect();
    }

    fn node_record_host(&self, nr: &NodeRecord) -> Option<Host> {
        self.hosts
            .iter()
            .find(|h| h.ipv6 == Ipv6Addr::from_str(&nr.http.clone().unwrap().ip_addr).unwrap())
            .cloned()
    }

    fn with_node_records(
        &self,
        records: &HashMap<PrincipalId, (RegistryVersion, NodeRecord)>,
    ) -> HashMap<PrincipalId, Node> {
        records
            .iter()
            // Skipping nodes without operator. This should only occur at version 1
            .filter(|(_, (_, nr))| !nr.node_operator_id.is_empty())
            .map(|(p, (version, nr))| {
                let host = self.node_record_host(nr);
                let operator = self
                    .operators
                    .iter()
                    .find(|(op, _)| op.to_vec() == nr.node_operator_id)
                    .map(|(_, o)| o.clone())
                    .unwrap_or_else(|| {
                        panic!(
                            "operator should exist in registry: node: {}, version: {}",
                            p, self.version
                        )
                    });
                (
                    *p,
                    Node {
                        principal: *p,
                        version: *version,
                        labels: host.as_ref().and_then(|h| h.labels.clone()).unwrap_or_default(),
                        ip_addr: node_ip_addr(nr),
                        hostname: host
                            .map(|h| h.name)
                            .unwrap_or_else(|| {
                                format!(
                                    "{}-{}",
                                    operator
                                        .datacenter
                                        .as_ref()
                                        .map(|d| d.name.clone())
                                        .unwrap_or_else(|| "??".to_string()),
                                    p.to_string().split_once('-').map(|(first, _)| first).unwrap_or("?????")
                                )
                            })
                            .into(),
                        subnet: self
                            .subnet_records
                            .iter()
                            .find(|(_, (_, sr))| sr.membership.contains(&p.to_vec()))
                            .map(|(p, _)| *p),
                        operator,
                        proposal: None,
                    },
                )
            })
            .collect()
    }

    fn update_nodes(&mut self) {
        self.nodes = self.with_node_records(&self.node_records);
    }

    fn update_removed_nodes(&mut self) {
        self.removed_nodes = self.with_node_records(
            &self
                .removed_node_records
                .clone()
                .into_iter()
                .filter(|(_, (_, rnr))| {
                    None == self
                        .node_records
                        .iter()
                        .find(|(_, (_, nr))| rnr.http.as_ref().unwrap().ip_addr == nr.http.as_ref().unwrap().ip_addr)
                })
                .collect(),
        );
    }

    fn update_subnets(&mut self) {
        self.subnets = self
            .subnet_records
            .iter()
            .map(|(p, (version, sr))| {
                let subnet_nodes = self
                    .nodes
                    .iter()
                    .filter(|(_, n)| n.subnet.map_or(false, |s| s == *p))
                    .map(|(_, n)| n.clone())
                    .collect::<Vec<Node>>();

                (
                    *p,
                    Subnet {
                        nodes: subnet_nodes,
                        principal: *p,
                        subnet_type: SubnetType::try_from(sr.subnet_type).unwrap(),
                        metadata: SubnetMetadata {
                            name: self.subnet_metadata.get(p).expect("record exists").name.clone(),
                            ..Default::default()
                        },
                        replica_version: sr.replica_version_id.clone(),
                        version: *version,
                        proposal: None,
                    },
                )
            })
            .filter(|(_, s)| !s.nodes.is_empty())
            .collect();
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn subnets(&self) -> HashMap<PrincipalId, Subnet> {
        self.subnets.clone()
    }

    pub fn nodes(&self) -> HashMap<PrincipalId, Node> {
        self.nodes.clone()
    }

    pub async fn nodes_with_proposals(&self) -> Result<HashMap<PrincipalId, Node>> {
        let nodes = self.nodes.clone();
        let proposal_agent = proposal::ProposalAgent::new();

        let mut topology_proposals = proposal_agent.list_valid_topology_proposals().await?;
        topology_proposals.reverse();
        let topology_proposals = topology_proposals;

        Ok(nodes
            .into_iter()
            .map(|(p, n)| {
                let last_associated_proposal = topology_proposals
                    .iter()
                    .find(|t| match &t.kind {
                        TopologyProposalKind::CreateSubnet(info) => info.nodes.contains(&n.principal),
                        TopologyProposalKind::ReplaceNode(info) => info.new_nodes.contains(&n.principal),
                    })
                    .cloned();

                // Return only the proposals that are still actionable
                let proposal = last_associated_proposal.and_then(|tp| {
                    if matches!(tp.status, TopologyProposalStatus::Open) {
                        Some(tp)
                    } else {
                        None
                    }
                });

                (p, Node { proposal, ..n })
            })
            .collect())
    }

    pub async fn subnets_with_proposals(&self) -> Result<HashMap<PrincipalId, Subnet>> {
        let subnets = self.subnets.clone();
        let proposal_agent = proposal::ProposalAgent::new();

        let topology_proposals = proposal_agent.list_valid_topology_proposals().await?;
        let topology_proposals = topology_proposals;

        Ok(subnets
            .into_iter()
            .map(|(p, subnet)| {
                let last_associated_proposal = topology_proposals
                    .iter()
                    .find(|t| match &t.kind {
                        TopologyProposalKind::ReplaceNode(info) => {
                            (!info.first
                                && info
                                    .new_nodes
                                    .iter()
                                    .any(|n| subnet.nodes.iter().any(|sn| sn.principal == *n)))
                                || (info.first
                                    && info
                                        .old_nodes
                                        .iter()
                                        .any(|n| subnet.nodes.iter().any(|sn| sn.principal == *n)))
                        }
                        _ => false,
                    })
                    .cloned();

                // Return only the proposals that are still actionable
                let proposal = last_associated_proposal.and_then(|tp| {
                    if matches!(tp.status, TopologyProposalStatus::Open) {
                        Some(tp)
                    } else {
                        match &tp.kind {
                            TopologyProposalKind::ReplaceNode(info)
                                if info.first
                                    // If the new nodes are not present in the subnet after the proposal has been executed, it means that replacement has been cancelled
                                    && info.new_nodes.iter().any(|n| {
                                        subnet.nodes.iter().any(|sn| sn.principal == *n)
                                    }) =>
                            {
                                Some(tp)
                            }
                            _ => None,
                        }
                    }
                });

                (p, Subnet { proposal, ..subnet })
            })
            .collect())
    }

    pub fn operators(&self) -> HashMap<PrincipalId, Operator> {
        self.operators.clone()
    }

    pub fn hosts(&self) -> Vec<Host> {
        self.hosts.clone()
    }

    pub fn removed_nodes(&self) -> HashMap<PrincipalId, Node> {
        self.removed_nodes.clone()
    }

    pub fn missing_hosts(&self) -> Vec<Host> {
        self.hosts
            .clone()
            .into_iter()
            .filter(|h| {
                !self
                    .nodes
                    .iter()
                    .any(|(_, n)| n.hostname.clone().unwrap_or_default() == h.name)
            })
            .collect()
    }

    pub fn replica_releases(&self) -> Vec<ReplicaRelease> {
        self.replica_releases.clone()
    }
}

impl decentralization::network::TopologyManager for RegistryState {}

#[async_trait]
impl SubnetQuerier for RegistryState {
    async fn subnet(&self, id: &PrincipalId) -> Result<decentralization::network::Subnet, NetworkError> {
        self.subnets
            .get(id)
            .map(|s| decentralization::network::Subnet {
                id: s.principal,
                nodes: s.nodes.iter().map(decentralization::network::Node::from).collect(),
            })
            .ok_or(NetworkError::SubnetNotFound(*id))
    }

    async fn subnet_of_nodes(&self, nodes: &[PrincipalId]) -> Result<decentralization::network::Subnet, NetworkError> {
        let subnets = nodes
            .to_vec()
            .iter()
            .map(|n| self.nodes.get(n).and_then(|n| n.subnet))
            .collect::<HashSet<_>>();
        if subnets.len() > 1 {
            return Err(NetworkError::IllegalRequest(
                "nodes don't belong to the same subnet".to_string(),
            ));
        }
        if let Some(Some(subnet)) = subnets.into_iter().next() {
            Ok(decentralization::network::Subnet {
                id: subnet,
                nodes: self
                    .subnets
                    .get(&subnet)
                    .ok_or(NetworkError::SubnetNotFound(subnet))?
                    .nodes
                    .iter()
                    .map(decentralization::network::Node::from)
                    .collect(),
            })
        } else {
            Err(NetworkError::IllegalRequest("no subnet found".to_string()))
        }
    }
}

#[async_trait]
impl AvailableNodesQuerier for RegistryState {
    async fn available_nodes(&self) -> Result<Vec<decentralization::network::Node>, NetworkError> {
        let nodes = self
            .nodes_with_proposals()
            .await
            .map_err(|_| NetworkError::DataRequestError)?
            .into_values()
            .filter(|n| n.subnet.is_none() && n.proposal.is_none())
            .collect::<Vec<_>>();
        let healths = crate::health::nodes()
            .await
            .map_err(|_| NetworkError::DataRequestError)?;
        Ok(nodes
            .iter()
            .filter(|n| {
                healths
                    .get(&n.principal)
                    .map(|s| matches!(*s, crate::health::Status::Healthy))
                    .unwrap_or(false)
            })
            .map(decentralization::network::Node::from)
            .sorted_by(|n1, n2| n1.id.cmp(&n2.id))
            .collect())
    }
}

#[derive(Clone)]
pub struct MetadataRegistry {
    counter: u64,
    system_subnet_counter: u64,
    metadatas: HashMap<PrincipalId, Metadata>,
}

impl MetadataRegistry {
    pub fn new() -> Self {
        Self {
            counter: 0,
            system_subnet_counter: 0,
            metadatas: HashMap::new(),
        }
    }

    pub fn get(&self, principal: &PrincipalId) -> Option<&Metadata> {
        self.metadatas.get(principal)
    }

    pub fn add(&mut self, principal: PrincipalId, subnet_type: SubnetType) {
        if self.metadatas.contains_key(&principal) {
            return;
        }

        const ROOT_SUBNETS_COUNT: u64 = 2;
        if self.system_subnet_counter >= ROOT_SUBNETS_COUNT {
            self.counter += 1;
        }
        self.metadatas.insert(
            principal,
            Metadata {
                name: match subnet_type {
                    SubnetType::System => {
                        self.system_subnet_counter += 1;
                        match self.system_subnet_counter {
                            1 => "Bootstrap NNS".to_string(),
                            2 => "NNS".to_string(),
                            3 => "People Parties".to_string(),
                            4 => "Internet Identity".to_string(),
                            _ => format!("System {}", self.system_subnet_counter),
                        }
                    }
                    SubnetType::Application | SubnetType::VerifiedApplication => {
                        format!("App {}", self.counter)
                    }
                },
            },
        );
    }

    pub fn remove(&mut self, principal: PrincipalId) {
        self.metadatas.remove(&principal);
    }
}

#[derive(Clone)]
pub struct Metadata {
    name: String,
}

pub(crate) enum PrincipalRecordValue {
    Subnet(Option<SubnetRecord>),
    Node(Option<NodeRecord>),
    Operator(Option<NodeOperatorRecord>),
}

pub struct PrincipalRecord {
    principal: PrincipalId,
    version: RegistryVersion,
    value: PrincipalRecordValue,
}

impl TryFrom<RegistryVersionedRecord<Vec<u8>>> for PrincipalRecord {
    type Error = &'static str;

    fn try_from(record: RegistryVersionedRecord<Vec<u8>>) -> Result<Self, Self::Error> {
        let key_prefix_end = record.key.rfind('_').ok_or("not a principal record")? + 1;
        Ok(PrincipalRecord {
            version: record.version,
            principal: PrincipalId::from_str(&record.key[key_prefix_end..]).map_err(|_| "cannot parse principal")?,
            value: match &record.key[..key_prefix_end] {
                SUBNET_RECORD_KEY_PREFIX => PrincipalRecordValue::Subnet(record_decode(record)),
                NODE_RECORD_KEY_PREFIX => PrincipalRecordValue::Node(record_decode(record)),
                NODE_OPERATOR_RECORD_KEY_PREFIX => PrincipalRecordValue::Operator(record_decode(record)),
                _ => return Err("unkown principal record"),
            },
        })
    }
}

fn record_decode<T: RegistryValue + Default>(record: RegistryVersionedRecord<Vec<u8>>) -> Option<T> {
    record
        .value
        .map(|value| RegistryValue::decode(value.as_slice()).expect("Record failed to decode"))
}

#[derive(Deserialize)]
struct Label {
    name: String,
    hosts: Vec<String>,
}

fn node_ip_addr(nr: &NodeRecord) -> Ipv6Addr {
    Ipv6Addr::from_str(&nr.http.clone().expect("missing ipv6 address").ip_addr).expect("invalid ipv6 address")
}
