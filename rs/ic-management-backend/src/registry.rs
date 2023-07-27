use crate::config::{nns_nodes_urls, target_network};
use crate::factsdb;
use crate::proposal;
use crate::public_dashboard::query_ic_dashboard_list;
use async_trait::async_trait;
use decentralization::network::{AvailableNodesQuerier, SubnetQuerier};
use futures::TryFutureExt;
use gitlab::api::AsyncQuery;
use gitlab::AsyncGitlab;
use ic_base_types::NodeId;
use ic_base_types::{RegistryVersion, SubnetId};
use ic_interfaces_registry::{RegistryClient, RegistryValue, ZERO_REGISTRY_VERSION};
use ic_management_types::{
    Datacenter, DatacenterOwner, Guest, Network, NetworkError, Node, NodeProviderDetails, NodeProvidersResponse,
    Operator, Provider, ReplicaRelease, Subnet, SubnetMetadata,
};
use ic_protobuf::registry::crypto::v1::PublicKey;
use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_protobuf::registry::unassigned_nodes_config::v1::UnassignedNodesConfigRecord;
use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord,
};
use ic_registry_client::client::ThresholdSigPublicKey;
use ic_registry_client_fake::FakeRegistryClient;
use ic_registry_client_helpers::{node::NodeRegistry, subnet::SubnetListRegistry};
use ic_registry_common_proto::pb::local_store::v1::{
    ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType,
};
use ic_registry_keys::DATA_CENTER_KEY_PREFIX;
use ic_registry_keys::{
    make_blessed_replica_versions_key, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX,
    SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_keys::{make_crypto_threshold_signing_pubkey_key, ROOT_SUBNET_ID_KEY};
use ic_registry_local_registry::LocalRegistry;
use ic_registry_local_store::{Changelog, ChangelogEntry, KeyMutation, LocalStoreImpl, LocalStoreWriter};
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::{debug, error, info, warn};
use registry_canister::mutations::common::decode_registry_value;
use std::convert::TryFrom;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv6Addr,
};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
extern crate env_logger;

use crate::gitlab::{CommitRef, CommitRefs};

use lazy_static::lazy_static;
use regex::Regex;

use anyhow::Result;

pub const NNS_SUBNET_NAME: &str = "NNS";

pub struct RegistryState {
    nns_url: String,
    network: Network,
    local_registry: Arc<LocalRegistry>,

    version: u64,
    subnets: BTreeMap<PrincipalId, Subnet>,
    nodes: BTreeMap<PrincipalId, Node>,
    operators: BTreeMap<PrincipalId, Operator>,
    guests: Vec<Guest>,
    known_subnets: BTreeMap<PrincipalId, String>,

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
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, T>>;
    fn get_family_entries_versioned<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, (u64, T)>>;
}

impl RegistryFamilyEntries for LocalRegistry {
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, T>> {
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
            .collect::<BTreeMap<_, _>>())
    }

    fn get_family_entries_versioned<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, (u64, T)>> {
        let prefix_length = T::KEY_PREFIX.len();
        Ok(self
            .get_key_family(T::KEY_PREFIX, self.get_latest_version())?
            .iter()
            .filter_map(|key| {
                self.get_versioned_value(key, self.get_latest_version())
                    .map(|r| {
                        r.value.as_ref().map(|v| {
                            (
                                key[prefix_length..].to_string(),
                                (
                                    r.version.get(),
                                    T::decode(v.as_slice()).expect("invalid registry value"),
                                ),
                            )
                        })
                    })
                    .unwrap_or_else(|_| panic!("failed to get entry {} for type {}", key, std::any::type_name::<T>()))
            })
            .collect::<BTreeMap<_, _>>())
    }
}

impl RegistryState {
    pub fn new(
        nns_url: String,
        network: Network,
        local_registry: Arc<LocalRegistry>,
        gitlab_client_public: Option<gitlab::AsyncGitlab>,
    ) -> Self {
        Self {
            nns_url,
            network,
            local_registry,
            version: 0,
            subnets: BTreeMap::<PrincipalId, Subnet>::new(),
            nodes: BTreeMap::new(),
            operators: BTreeMap::new(),
            guests: Vec::new(),
            replica_releases: Vec::new(),
            gitlab_client_public,
            known_subnets: [
                (
                    "uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe",
                    "Internet Identity, tECDSA backup",
                ),
                (
                    "w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe",
                    "Bitcoin",
                ),
                (
                    "eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe",
                    "Open Chat 1",
                ),
                (
                    "2fq7c-slacv-26cgz-vzbx2-2jrcs-5edph-i5s2j-tck77-c3rlz-iobzx-mqe",
                    "Open Chat 2",
                ),
                (
                    "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae",
                    "tECDSA signing",
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

    pub async fn update(&mut self, providers: Vec<NodeProviderDetails>, guests: Vec<Guest>) -> anyhow::Result<()> {
        self.guests = guests;
        if !matches!(self.network, Network::Mainnet) {
            for g in &mut self.guests {
                g.dfinity_owned = true;
            }
        }
        self.local_registry
            .sync_with_local_store()
            .await
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
                    &make_blessed_replica_versions_key(),
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
                            Err(anyhow::anyhow!("unable to parse release name for version {}, refs {:?}", version, refs))
                        }
                    }
                    Err(gitlab::api::ApiError::Gitlab { msg }) if msg.contains(reqwest::StatusCode::NOT_FOUND.as_str()) => Err(anyhow::format_err!("no releases found for version {}", version)),
                    Err(e) => Err(anyhow::format_err!("query failed: {}", e)),
                }
            }).collect::<Vec<_>>();
            if let Some(Err(e)) = new_blessed_versions.iter().find(|r| r.is_err()) {
                return Err(anyhow::anyhow!("failed to query gitlab for blessed versions: {}", e));
            }

            new_blessed_versions
                .into_iter()
                .map(|r| r.unwrap())
                .for_each(|mut nrr| {
                    nrr.previous_patch_release = self
                        .replica_releases
                        .iter()
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
            .collect::<BTreeMap<_, _>>();
        let data_center_records: BTreeMap<String, DataCenterRecord> = self.local_registry.get_family_entries()?;
        let operator_records: BTreeMap<String, NodeOperatorRecord> = self.local_registry.get_family_entries()?;

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
        let node_entries = self.local_registry.get_family_entries_versioned::<NodeRecord>()?;
        self.nodes = node_entries
            .iter()
            // Skipping nodes without operator. This should only occur at version 1
            .filter(|(_, (_, nr))| !nr.node_operator_id.is_empty())
            .map(|(p, (_, nr))| {
                let guest = self.node_record_guest(nr);
                let operator = self
                    .operators
                    .iter()
                    .find(|(op, _)| op.to_vec() == nr.node_operator_id)
                    .map(|(_, o)| o.clone())
                    .expect("missing operator referenced by a node");
                let principal = PrincipalId::from_str(p).expect("invalid node principal id");
                let ip_addr = node_ip_addr(nr);
                (
                    principal,
                    Node {
                        principal,
                        dfinity_owned: Some(guest.as_ref().map(|g| g.dfinity_owned).unwrap_or_default()),
                        ip_addr,
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
                        subnet_id: self
                            .local_registry
                            .get_subnet_id_from_node_id(
                                NodeId::new(principal),
                                self.local_registry.get_latest_version(),
                            )
                            .expect("failed to get subnet id")
                            .map(|s| s.get()),
                        operator,
                        proposal: None,
                        label: guest.map(|g| g.name),
                        decentralized: ip_addr.segments()[4] == 0x6801,
                        duplicates: node_entries
                            .iter()
                            .filter(|(_, (_, nr2))| node_ip_addr(nr2) == node_ip_addr(nr))
                            .max_by_key(|(_, (version, _))| version)
                            .and_then(|(p2, _)| {
                                if p2 == p {
                                    None
                                } else {
                                    Some(PrincipalId::from_str(p2).expect("invalid node principal id"))
                                }
                            }),
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
                    .filter(|(_, n)| n.subnet_id.map_or(false, |s| s == principal))
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

    pub fn network(&self) -> Network {
        self.network.clone()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn subnets(&self) -> BTreeMap<PrincipalId, Subnet> {
        self.subnets.clone()
    }

    pub fn nodes(&self) -> BTreeMap<PrincipalId, Node> {
        self.nodes.clone()
    }

    pub async fn nodes_with_proposals(&self) -> Result<BTreeMap<PrincipalId, Node>> {
        let nodes = self.nodes.clone();
        let proposal_agent = proposal::ProposalAgent::new(self.nns_url.clone());

        let topology_proposals = proposal_agent.list_open_topology_proposals().await?;

        Ok(nodes
            .into_iter()
            .map(|(p, n)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|p| p.node_ids_added.contains(&n.principal) || p.node_ids_removed.contains(&n.principal))
                    .cloned();

                (p, Node { proposal, ..n })
            })
            .collect())
    }

    pub async fn subnets_with_proposals(&self) -> Result<BTreeMap<PrincipalId, Subnet>> {
        let subnets = self.subnets.clone();
        let proposal_agent = proposal::ProposalAgent::new(self.nns_url.clone());

        let topology_proposals = proposal_agent.list_open_topology_proposals().await?;

        Ok(subnets
            .into_iter()
            .map(|(subnet_id, subnet)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|p| {
                        p.subnet_id.unwrap_or_default() == subnet_id
                            || subnet.nodes.iter().any(|n| {
                                p.node_ids_added.contains(&n.principal) || p.node_ids_removed.contains(&n.principal)
                            })
                    })
                    .cloned();

                (subnet_id, Subnet { proposal, ..subnet })
            })
            .collect())
    }

    pub async fn retireable_versions(&self) -> Result<Vec<ReplicaRelease>> {
        const NUM_RELEASE_BRANCHES_TO_KEEP: usize = 2;
        let active_releases = self
            .replica_releases
            .clone()
            .into_iter()
            .rev()
            .map(|rr| rr.branch)
            .unique()
            .take(NUM_RELEASE_BRANCHES_TO_KEEP)
            .collect::<Vec<_>>();
        let subnet_versions: BTreeSet<String> = self.subnets.values().map(|s| s.replica_version.clone()).collect();
        let version_on_unassigned_nodes = self.get_unassigned_nodes_version().await?;
        Ok(self
            .replica_releases
            .clone()
            .into_iter()
            .filter(|rr| !active_releases.contains(&rr.branch))
            .filter(|rr| !subnet_versions.contains(&rr.commit_hash) && rr.commit_hash != version_on_unassigned_nodes)
            .collect())
    }

    pub async fn nns_replica_version(&self) -> Option<String> {
        Some(
            self.subnets()
                .get(
                    &PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe").unwrap(),
                )?
                .replica_version
                .clone(),
        )
    }

    pub fn operators(&self) -> BTreeMap<PrincipalId, Operator> {
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

    pub async fn get_unassigned_nodes_version(&self) -> Result<String, anyhow::Error> {
        let unassigned_config_key = ic_registry_keys::make_unassigned_nodes_config_record_key();

        match self
            .local_registry
            .get_value(&unassigned_config_key, self.local_registry.get_latest_version())
        {
            Ok(Some(bytes)) => {
                let cfg = UnassignedNodesConfigRecord::decode(&bytes[..])
                    .expect("Error decoding UnassignedNodesConfigRecord from the LocalRegistry");

                Ok(cfg.replica_version)
            }
            _ => Err(anyhow::anyhow!(
                "No replica version for unassigned nodes found".to_string(),
            )),
        }
    }
}

impl decentralization::network::TopologyManager for RegistryState {}

#[async_trait]
impl SubnetQuerier for RegistryState {
    async fn subnet(&self, id: &PrincipalId) -> Result<decentralization::network::DecentralizedSubnet, NetworkError> {
        self.subnets
            .get(id)
            .map(|s| decentralization::network::DecentralizedSubnet {
                id: s.principal,
                nodes: s.nodes.iter().map(decentralization::network::Node::from).collect(),
                removed_nodes: Vec::new(),
                min_nakamoto_coefficients: None,
                comment: None,
                run_log: Vec::new(),
            })
            .ok_or(NetworkError::SubnetNotFound(*id))
    }

    async fn subnet_of_nodes(
        &self,
        nodes: Vec<decentralization::network::Node>,
    ) -> Result<decentralization::network::DecentralizedSubnet, NetworkError> {
        let subnets = nodes
            .to_vec()
            .iter()
            .map(|n| self.nodes.get(&n.id).and_then(|n| n.subnet_id))
            .collect::<BTreeSet<_>>();
        if subnets.len() > 1 {
            return Err(NetworkError::IllegalRequest(
                "nodes don't belong to the same subnet".to_string(),
            ));
        }
        if let Some(Some(subnet)) = subnets.into_iter().next() {
            Ok(decentralization::network::DecentralizedSubnet {
                id: subnet,
                nodes: self
                    .subnets
                    .get(&subnet)
                    .ok_or(NetworkError::SubnetNotFound(subnet))?
                    .nodes
                    .iter()
                    .map(decentralization::network::Node::from)
                    .collect(),
                removed_nodes: Vec::new(),
                min_nakamoto_coefficients: None,
                comment: None,
                run_log: Vec::new(),
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
            .filter(|n| n.subnet_id.is_none() && n.proposal.is_none() && n.duplicates.is_none())
            .collect::<Vec<_>>();

        let health_client = crate::health::HealthClient::new(self.network());
        let healths = health_client
            .nodes()
            .await
            .map_err(|_| NetworkError::DataRequestError)?;
        Ok(nodes
            .iter()
            .filter(|n| {
                // Keep only healthy nodes.
                healths
                    .get(&n.principal)
                    .map(|s| matches!(*s, ic_management_types::Status::Healthy))
                    .unwrap_or(false)
            })
            .filter(|n| {
                // Keep only the decentralized or DFINITY-owned nodes.
                n.decentralized || n.dfinity_owned.unwrap_or(false)
            })
            .map(decentralization::network::Node::from)
            .sorted_by(|n1, n2| n1.id.cmp(&n2.id))
            .collect())
    }
}

fn node_ip_addr(nr: &NodeRecord) -> Ipv6Addr {
    Ipv6Addr::from_str(&nr.http.clone().expect("missing ipv6 address").ip_addr).expect("invalid ipv6 address")
}

pub fn local_registry_path(network: Network) -> PathBuf {
    match std::env::var("LOCAL_REGISTRY_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => match dirs::cache_dir() {
            Some(cache_dir) => cache_dir,
            None => PathBuf::from("/tmp"),
        },
    }
    .join("ic-registry-cache")
    .join(Path::new(network.to_string().as_str()))
    .join("local_registry")
}

pub async fn nns_public_key(registry_canister: &RegistryCanister) -> anyhow::Result<ThresholdSigPublicKey> {
    let (nns_subnet_id_vec, _) = registry_canister
        .get_value(ROOT_SUBNET_ID_KEY.as_bytes().to_vec(), None)
        .await
        .map_err(|e| anyhow::format_err!("failed to get root subnet: {}", e))?;
    let nns_subnet_id = decode_registry_value::<ic_protobuf::types::v1::SubnetId>(nns_subnet_id_vec);
    let (nns_pub_key_vec, _) = registry_canister
        .get_value(
            make_crypto_threshold_signing_pubkey_key(SubnetId::new(
                PrincipalId::try_from(nns_subnet_id.principal_id.unwrap().raw).unwrap(),
            ))
            .as_bytes()
            .to_vec(),
            None,
        )
        .await
        .map_err(|e| anyhow::format_err!("failed to get public key: {}", e))?;
    Ok(
        ThresholdSigPublicKey::try_from(PublicKey::decode(nns_pub_key_vec.as_slice()).expect("invalid public key"))
            .expect("failed to create thresholdsig public key"),
    )
}

pub async fn sync_local_store(target_network: Network) -> anyhow::Result<()> {
    let local_registry_path = local_registry_path(target_network);
    let local_store = Arc::new(LocalStoreImpl::new(local_registry_path.clone()));
    let registry_canister = RegistryCanister::new(nns_nodes_urls());
    let mut latest_version = if !Path::new(&local_registry_path).exists() {
        ZERO_REGISTRY_VERSION
    } else {
        let registry_cache = FakeRegistryClient::new(local_store.clone());
        registry_cache.update_to_latest_version();
        registry_cache.get_latest_version()
    };
    info!("Syncing local registry from version {}", latest_version);
    let mut latest_certified_time = 0;
    let mut updates = vec![];
    let nns_public_key = nns_public_key(&registry_canister).await?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if match registry_canister.get_latest_version().await {
            Ok(v) => {
                info!("Latest registry version: {}", v);
                v == latest_version.get()
            }
            Err(e) => {
                error!("Failed to get latest registry version: {}", e);
                false
            }
        } {
            break;
        }
        if let Ok((mut initial_records, _, t)) = registry_canister
            .get_certified_changes_since(latest_version.get(), &nns_public_key)
            .await
        {
            initial_records.sort_by_key(|tr| tr.version);
            let changelog = initial_records.iter().fold(Changelog::default(), |mut cl, r| {
                let rel_version = (r.version - latest_version).get();
                if cl.len() < rel_version as usize {
                    cl.push(ChangelogEntry::default());
                }
                cl.last_mut().unwrap().push(KeyMutation {
                    key: r.key.clone(),
                    value: r.value.clone(),
                });
                cl
            });

            let versions_count = changelog.len();

            changelog.into_iter().enumerate().for_each(|(i, ce)| {
                let v = RegistryVersion::from(i as u64 + 1 + latest_version.get());
                let local_registry_path = local_registry_path.clone();
                updates.push(async move {
                    let path_str = format!("{:016x}.pb", v.get());
                    // 00 01 02 03 04 / 05 / 06 / 07.pb
                    let v_path = &[
                        &path_str[0..10],
                        &path_str[10..12],
                        &path_str[12..14],
                        &path_str[14..19],
                    ]
                    .iter()
                    .collect::<PathBuf>();
                    let path = local_registry_path.join(v_path.as_path());
                    let r = tokio::fs::create_dir_all(path.clone().parent().unwrap())
                        .and_then(|_| async {
                            tokio::fs::write(
                                path,
                                PbChangelogEntry {
                                    key_mutations: ce
                                        .iter()
                                        .map(|km| {
                                            let mutation_type = if km.value.is_some() {
                                                MutationType::Set as i32
                                            } else {
                                                MutationType::Unset as i32
                                            };
                                            PbKeyMutation {
                                                key: km.key.clone(),
                                                value: km.value.clone().unwrap_or_default(),
                                                mutation_type,
                                            }
                                        })
                                        .collect(),
                                }
                                .encode_to_vec(),
                            )
                            .await
                        })
                        .await;
                    if let Err(e) = &r {
                        debug!("Storage err for {v}: {}", e);
                    } else {
                        debug!("Stored version {}", v);
                    }
                    r
                });
            });

            latest_version = latest_version.add(RegistryVersion::new(versions_count as u64));

            latest_certified_time = t.as_nanos_since_unix_epoch();
            debug!("Sync reached version {latest_version}");
        }
    }

    futures::future::join_all(updates).await;
    local_store.update_certified_time(latest_certified_time)?;
    Ok(())
}

pub async fn poll(gitlab_client: AsyncGitlab, registry_state: Arc<RwLock<RegistryState>>) {
    let mut print_counter = 0;
    let registry_canister = RegistryCanister::new(nns_nodes_urls());
    loop {
        sleep(Duration::from_secs(1)).await;
        let print_enabled = print_counter % 10 == 0;
        if print_enabled {
            info!("Updating registry");
        }
        let latest_version = if let Ok(v) = registry_canister.get_latest_version().await {
            v
        } else {
            continue;
        };
        if latest_version != registry_state.read().await.version() {
            let node_providers_result = query_ic_dashboard_list::<NodeProvidersResponse>("v3/node-providers").await;
            let network = target_network();
            let guests_result = factsdb::query_guests(gitlab_client.clone(), network.to_string()).await;
            let guests_result = if matches!(network, Network::Mainnet) {
                let guests_result_old =
                    factsdb::query_guests(gitlab_client.clone(), network.legacy_name().to_string()).await;
                guests_result.and_then(|guests_decentralized| {
                    guests_result_old.map(|guests_old| {
                        guests_decentralized
                            .into_iter()
                            .chain(guests_old.into_iter())
                            .collect::<Vec<_>>()
                    })
                })
            } else {
                guests_result
            };
            match (node_providers_result, guests_result) {
                (Ok(node_providers_response), Ok(guests_list)) => {
                    let mut registry_state = registry_state.write().await;
                    let update = registry_state
                        .update(node_providers_response.node_providers, guests_list)
                        .await;
                    if let Err(e) = update {
                        warn!("failed state update: {}", e);
                    }
                    if print_enabled {
                        info!("Updated registry state to version {}", registry_state.version());
                    }
                }
                (Err(e), _) => {
                    warn!("Failed querying IC dashboard {}", e);
                }
                (_, Err(e)) => {
                    warn!("Failed querying guests file: {}", e);
                }
            }
        } else if print_enabled {
            info!(
                "Skipping update. Registry already on latest version: {}",
                registry_state.read().await.version()
            )
        }
        print_counter += 1;
    }
}
