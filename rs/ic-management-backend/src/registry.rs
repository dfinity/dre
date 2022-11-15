use crate::proposal;
use async_trait::async_trait;
use decentralization::network::{AvailableNodesQuerier, SubnetQuerier};
use ic_base_types::NodeId;
use ic_interfaces_registry::RegistryValue;
use ic_management_types::{
    Datacenter, DatacenterOwner, Guest, NetworkError, Node, NodeProviderDetails, Operator, Provider, ReplicaRelease,
    Subnet, SubnetMetadata,
};
use ic_registry_keys::{
    make_blessed_replica_version_key, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_local_registry::LocalRegistry;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use std::convert::TryFrom;
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    net::Ipv6Addr,
};

use ic_interfaces_registry::RegistryClient;
use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord,
};
use ic_registry_client_helpers::{node::NodeRegistry, subnet::SubnetListRegistry};

use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_registry_keys::DATA_CENTER_KEY_PREFIX;

use std::str::FromStr;

use crate::gitlab::{CommitRef, CommitRefs};
use gitlab::api::AsyncQuery;

use lazy_static::lazy_static;
use regex::Regex;

use anyhow::Result;

pub const NNS_SUBNET_NAME: &str = "NNS";

pub struct RegistryState {
    nns_url: String,
    network: String,
    local_registry: Arc<LocalRegistry>,

    version: u64,
    subnets: HashMap<PrincipalId, Subnet>,
    nodes: HashMap<PrincipalId, Node>,
    operators: HashMap<PrincipalId, Operator>,
    guests: Vec<Guest>,
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
    pub(crate) fn new(
        nns_url: String,
        network: String,
        local_registry: Arc<LocalRegistry>,
        gitlab_client_public: Option<gitlab::AsyncGitlab>,
    ) -> Self {
        Self {
            nns_url,
            network,
            local_registry,
            version: 0,
            subnets: HashMap::<PrincipalId, Subnet>::new(),
            nodes: HashMap::new(),
            operators: HashMap::new(),
            guests: Vec::new(),
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
                ("x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae", "SNS"),
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

    pub(crate) async fn update(
        &mut self,
        providers: Vec<NodeProviderDetails>,
        guests: Vec<Guest>,
    ) -> anyhow::Result<()> {
        self.guests = guests;
        if self.network == "staging" {
            for g in &mut self.guests {
                g.dfinity_owned = true;
            }
        }
        self.local_registry
            .sync_with_local_store()
            .map_err(|e| anyhow::anyhow!(e))?;
        self.update_replica_releases().await?;
        self.update_operators(providers)?;
        self.update_nodes()?;
        self.update_subnets()?;
        self.version = self.local_registry.get_latest_version().get();

        Ok(())
    }

    async fn update_replica_releases(&mut self) -> Result<()> {
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
                    .rfind(|rr| rr.name == nrr.name && rr.commit_hash != nrr.commit_hash)
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

    fn update_operators(&mut self, providers: Vec<NodeProviderDetails>) -> Result<()> {
        let providers = providers
            .into_iter()
            .map(|p| (p.principal_id, p))
            .collect::<HashMap<_, _>>();
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
                                owner: DatacenterOwner { name: dc.owner.clone() },
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

    fn node_record_guest(&self, nr: &NodeRecord) -> Option<Guest> {
        self.guests
            .iter()
            .find(|g| g.ipv6 == Ipv6Addr::from_str(&nr.http.clone().unwrap().ip_addr).unwrap())
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
                let guest = self.node_record_guest(nr);
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
                        dfinity_owned: Some(guest.as_ref().map(|g| g.dfinity_owned).unwrap_or_default()),
                        ip_addr: node_ip_addr(nr),
                        hostname: guest
                            .as_ref()
                            .map(|g| g.name.clone())
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
                        label: guest.clone().map(|g| g.name),
                        decentralized: guest.map(|g| g.decentralized).unwrap_or_default(),
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
                                            NNS_SUBNET_NAME.to_string()
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
                        replica_release: self
                            .replica_releases
                            .iter()
                            .find(|r| r.commit_hash == sr.replica_version_id)
                            .cloned(),
                        proposal: None,
                    },
                )
            })
            .filter(|(_, s)| !s.nodes.is_empty())
            .collect();

        Ok(())
    }

    pub fn network(&self) -> String {
        self.network.clone()
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
        let proposal_agent = proposal::ProposalAgent::new(self.nns_url.clone());

        let topology_proposals = proposal_agent.list_open_topology_proposals().await?;

        Ok(nodes
            .into_iter()
            .map(|(p, n)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|p| p.nodes.contains(&n.principal))
                    .cloned();

                (p, Node { proposal, ..n })
            })
            .collect())
    }

    pub async fn subnets_with_proposals(&self) -> Result<HashMap<PrincipalId, Subnet>> {
        let subnets = self.subnets.clone();
        let proposal_agent = proposal::ProposalAgent::new(self.nns_url.clone());

        let topology_proposals = proposal_agent.list_open_topology_proposals().await?;

        Ok(subnets
            .into_iter()
            .map(|(p, subnet)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|t| {
                        t.subnet_id.unwrap_or_default() == p
                            || subnet.nodes.iter().any(|n| t.nodes.contains(&n.principal))
                    })
                    .cloned();

                (p, Subnet { proposal, ..subnet })
            })
            .collect())
    }

    pub fn operators(&self) -> HashMap<PrincipalId, Operator> {
        self.operators.clone()
    }

    pub fn guests(&self) -> Vec<Guest> {
        self.guests.clone()
    }

    pub fn missing_guests(&self) -> Vec<Guest> {
        let mut missing_guests = self
            .guests
            .clone()
            .into_iter()
            .filter(|g| {
                !self
                    .nodes
                    .iter()
                    .any(|(_, n)| n.label.clone().unwrap_or_default() == g.name)
            })
            .collect::<Vec<_>>();
        missing_guests.sort_by_key(|g| g.name.clone());
        missing_guests.dedup_by_key(|g| g.name.clone());
        missing_guests
    }

    pub fn replica_releases(&self) -> Vec<ReplicaRelease> {
        self.replica_releases.clone()
    }

    pub fn nns_url(&self) -> String {
        self.nns_url.clone()
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

        let health_client = crate::health::HealthClient::new(self.network());
        let healths = health_client
            .nodes()
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

fn node_ip_addr(nr: &NodeRecord) -> Ipv6Addr {
    Ipv6Addr::from_str(&nr.http.clone().expect("missing ipv6 address").ip_addr).expect("invalid ipv6 address")
}
