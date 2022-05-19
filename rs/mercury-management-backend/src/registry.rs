use crate::proposal;
use async_trait::async_trait;
use decentralization::network::{AvailableNodesQuerier, NetworkError, SubnetQuerier};
use ic_base_types::NodeId;
use ic_interfaces::registry::RegistryValue;
use ic_registry_client::local_registry::LocalRegistry;
use ic_registry_keys::{
    make_blessed_replica_version_key, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use mercury_management_types::{
    Datacenter, DatacenterOwner, Host, Location, Node, NodeLabel, NodeLabelName, Operator, Provider, ProviderDetails,
    ReplicaRelease, Subnet, SubnetMetadata, TopologyProposalKind, TopologyProposalStatus,
};
use std::convert::TryFrom;
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    net::Ipv6Addr,
};

use ic_interfaces::registry::RegistryClient;
use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord,
};
use ic_registry_client_helpers::{node::NodeRegistry, subnet::SubnetListRegistry};

use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_registry_keys::DATA_CENTER_KEY_PREFIX;

use serde::Deserialize;
use std::str::FromStr;

use crate::gitlab::{CommitRef, CommitRefs};
use gitlab::api::AsyncQuery;

use lazy_static::lazy_static;
use regex::Regex;

use anyhow::Result;

pub struct RegistryState {
    local_registry: Arc<LocalRegistry>,

    version: u64,
    subnets: HashMap<PrincipalId, Subnet>,
    nodes: HashMap<PrincipalId, Node>,
    operators: HashMap<PrincipalId, Operator>,
    hosts: Vec<Host>,
    known_subnets: HashMap<PrincipalId, String>,

    replica_releases: Vec<ReplicaRelease>,
    gitlab_client_public: Option<gitlab::AsyncGitlab>,
}
trait RegistryEntry: RegistryValue {
    const KEY_PREFIX: &'static str;
}

impl RegistryEntry for DataCenterRecord {
    const KEY_PREFIX: &'static str = DATA_CENTER_KEY_PREFIX;
}

impl RegistryEntry for NodeOperatorRecord {
    const KEY_PREFIX: &'static str = NODE_OPERATOR_RECORD_KEY_PREFIX;
}

impl RegistryEntry for NodeRecord {
    const KEY_PREFIX: &'static str = NODE_RECORD_KEY_PREFIX;
}

impl RegistryEntry for SubnetRecord {
    const KEY_PREFIX: &'static str = SUBNET_RECORD_KEY_PREFIX;
}

trait RegistryFamilyEntries {
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<HashMap<String, T>>;
}

impl RegistryFamilyEntries for LocalRegistry {
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<HashMap<String, T>> {
        let prefix_length = T::KEY_PREFIX.len();
        Ok(self
            .get_key_family(T::KEY_PREFIX, self.get_latest_version())?
            .iter()
            .filter_map(|key| {
                self.get_value(key, self.get_latest_version())
                    .unwrap_or_else(|_| panic!("failed to get entry {} for type {}", key, std::any::type_name::<T>()))
                    .map(|v| {
                        (
                            key[prefix_length..].to_string(),
                            T::decode(v.as_slice()).expect("invalid registry value"),
                        )
                    })
            })
            .collect::<HashMap<_, _>>())
    }
}

impl RegistryState {
    pub(crate) fn new(local_registry: Arc<LocalRegistry>, gitlab_client_public: Option<gitlab::AsyncGitlab>) -> Self {
        let labels =
            serde_yaml::from_str::<Vec<Label>>(include_str!("../data/labels.yaml")).expect("invalid configuration");

        Self {
            local_registry,
            version: 0,
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
            replica_releases: Vec::new(),
            gitlab_client_public,
            known_subnets: [
                (
                    "uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe",
                    "Internet Identity",
                ),
                (
                    "w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe",
                    "People Parties",
                ),
                (
                    "2fq7c-slacv-26cgz-vzbx2-2jrcs-5edph-i5s2j-tck77-c3rlz-iobzx-mqe",
                    "Bitcoin",
                ),
            ]
            .iter()
            .map(|(p, name)| {
                (
                    PrincipalId::from_str(p).expect("invalid principal id"),
                    name.to_string(),
                )
            })
            .collect(),
        }
    }

    pub(crate) fn sycned(&self) -> bool {
        self.version == self.local_registry.get_latest_version().get()
    }

    pub(crate) async fn update(
        &mut self,
        locations: Vec<Location>,
        providers: Vec<ProviderDetails>,
    ) -> anyhow::Result<()> {
        let locations = locations
            .into_iter()
            .map(|l| (l.key.clone(), l))
            .collect::<HashMap<_, _>>();
        let providers = providers
            .into_iter()
            .map(|p| (p.principal_id, p))
            .collect::<HashMap<_, _>>();

        self.update_replica_releases().await?;
        self.update_operators(locations, providers)?;
        self.update_nodes()?;
        self.update_subnets()?;
        self.version = self.local_registry.get_latest_version().get();

        Ok(())
    }

    async fn update_replica_releases(&mut self) -> Result<()> {
        const STARTING_VERSION: &str = "0ef2aebde4ff735a1a93efa342dcf966b6df5061";
        let blessed_versions = BlessedReplicaVersions::decode(
            self.local_registry
                .get_value(
                    &make_blessed_replica_version_key(),
                    self.local_registry.get_latest_version(),
                )?
                .unwrap_or_default()
                .as_slice(),
        )
        .expect("failed to decode blessed replica versions")
        .blessed_version_ids;

        if let Some(gitlab_client_public) = &self.gitlab_client_public {
            let new_blessed_versions = futures::future::join_all(
                blessed_versions
            .iter()
            .skip_while(|v| *v != STARTING_VERSION)
            .filter(|v| !self.replica_releases.iter().any(|rr| rr.commit_hash == **v)).map(|version| {
                    let endpoint_public = CommitRefs::builder()
                        .project("dfinity-lab/public/ic")
                        .commit(version)
                        .build()
                        .expect("unable to build refs query");

                    async move {
                        let refs: Result<Vec<CommitRef>, _> = gitlab::api::paged(endpoint_public, gitlab::api::Pagination::All).query_async(gitlab_client_public).await;
                        (version, refs)
                    }
            }).collect::<Vec<_>>()).await.into_iter().map(|(version, refs_result)| {
                match refs_result {
                    Ok(refs) => {
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
                                previous_patch_release: None,
                                time: chrono::NaiveDateTime::parse_from_str(
                                    captures
                                        .name(&DATETIME_NAME_GROUP)
                                        .expect("release regex misconfiguration")
                                        .as_str(),
                                    "%Y-%m-%d_%H-%M",
                                )
                                .expect("invalid datetime format"),
                            };
                            Ok(rr)
                        } else {
                            Err(anyhow::anyhow!("unable to parse release name"))
                        }
                    }
                    Err(gitlab::api::ApiError::Gitlab { msg }) if msg.contains(reqwest::StatusCode::NOT_FOUND.as_str()) => Err(anyhow::format_err!("no releases found for version {}", version)),
                    Err(e) => Err(anyhow::format_err!("query failed: {}", e)),
                }
            }).collect::<Vec<_>>();
            if let Some(Err(e)) = new_blessed_versions.iter().find(|r| r.is_err()) {
                return Err(anyhow::anyhow!("failed to query gitlab for blessed versions: {}", e));
            }

            let new_blessed_versions = new_blessed_versions.into_iter().map(|r| r.unwrap()).collect::<Vec<_>>();
            new_blessed_versions.clone().into_iter().for_each(|mut nrr| {
                nrr.previous_patch_release = self
                    .replica_releases
                    .iter()
                    .chain(new_blessed_versions.clone().iter())
                    .rfind(|rr| rr.name == nrr.name)
                    .map(|rr| rr.clone().into());
                self.replica_releases.push(nrr);
            });
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
    ) -> Result<()> {
        let data_center_records: HashMap<String, DataCenterRecord> = self.local_registry.get_family_entries()?;
        let operator_records: HashMap<String, NodeOperatorRecord> = self.local_registry.get_family_entries()?;

        self.operators = operator_records
            .iter()
            .map(|(p, or)| {
                let principal = PrincipalId::from_str(p).expect("invalid operator principal id");
                (
                    principal,
                    Operator {
                        principal,
                        provider: PrincipalId::try_from(&or.node_provider_principal_id[..])
                            .map(|p| Provider {
                                name: providers.get(&p).map(|pd| pd.display_name.clone()),
                                website: providers.get(&p).and_then(|pd| pd.website.clone()),
                                principal: p,
                            })
                            .expect("provider missing from operator record"),
                        allowance: or.node_allowance,
                        datacenter: data_center_records.get(&or.dc_id).map(|dc| {
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

        Ok(())
    }

    fn node_record_host(&self, nr: &NodeRecord) -> Option<Host> {
        self.hosts
            .iter()
            .find(|h| h.ipv6 == Ipv6Addr::from_str(&nr.http.clone().unwrap().ip_addr).unwrap())
            .cloned()
    }

    fn update_nodes(&mut self) -> Result<()> {
        self.nodes = self
            .local_registry
            .get_family_entries::<NodeRecord>()?
            .iter()
            // Skipping nodes without operator. This should only occur at version 1
            .filter(|(_, nr)| !nr.node_operator_id.is_empty())
            .map(|(p, nr)| {
                let host = self.node_record_host(nr);
                let operator = self
                    .operators
                    .iter()
                    .find(|(op, _)| op.to_vec() == nr.node_operator_id)
                    .map(|(_, o)| o.clone())
                    .expect("missing operator referenced by a node");
                let principal = PrincipalId::from_str(p).expect("invalid node principal id");
                (
                    principal,
                    Node {
                        principal,
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
                            .local_registry
                            .get_subnet_id_from_node_id(
                                NodeId::new(principal),
                                self.local_registry.get_latest_version(),
                            )
                            .expect("failed to get subnet id")
                            .map(|s| s.get()),
                        operator,
                        proposal: None,
                    },
                )
            })
            .collect();

        Ok(())
    }

    fn update_subnets(&mut self) -> Result<()> {
        self.subnets = self
            .local_registry
            .get_family_entries::<SubnetRecord>()?
            .iter()
            .map(|(p, sr)| {
                let principal = PrincipalId::from_str(p).expect("invalid subnet principal id");
                let subnet_nodes = self
                    .nodes
                    .iter()
                    .filter(|(_, n)| n.subnet.map_or(false, |s| s == principal))
                    .map(|(_, n)| n.clone())
                    .collect::<Vec<Node>>();
                let subnet_type = SubnetType::try_from(sr.subnet_type).unwrap();

                (
                    principal,
                    Subnet {
                        nodes: subnet_nodes,
                        principal,
                        subnet_type,
                        metadata: SubnetMetadata {
                            name: if let Some(name) = self.known_subnets.get(&principal) {
                                name.clone()
                            } else {
                                self.local_registry
                                    .get_subnet_ids(self.local_registry.get_latest_version())
                                    .expect("failed to list subnets")
                                    .unwrap_or_default()
                                    .iter()
                                    .position(|s| s.get() == principal)
                                    .map(|i| {
                                        if i == 0 {
                                            "NNS".to_string()
                                        } else {
                                            format!(
                                                "{} {}",
                                                match subnet_type {
                                                    SubnetType::Application | SubnetType::VerifiedApplication => "App",
                                                    SubnetType::System => "System",
                                                },
                                                i
                                            )
                                        }
                                    })
                                    .unwrap_or_else(|| "??".to_string())
                            },
                            ..Default::default()
                        },
                        replica_version: sr.replica_version_id.clone(),
                        proposal: None,
                    },
                )
            })
            .filter(|(_, s)| !s.nodes.is_empty())
            .collect();

        Ok(())
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

#[derive(Deserialize)]
struct Label {
    name: String,
    hosts: Vec<String>,
}

fn node_ip_addr(nr: &NodeRecord) -> Ipv6Addr {
    Ipv6Addr::from_str(&nr.http.clone().expect("missing ipv6 address").ip_addr).expect("invalid ipv6 address")
}
