use crate::git_ic_repo::IcRepo;
use crate::health::HealthStatusQuerier;
use crate::node_labels;
use crate::proposal::{self, SubnetUpdateProposal, UpdateUnassignedNodesProposal};
use crate::public_dashboard::query_ic_dashboard_list;
use async_trait::async_trait;
use decentralization::network::{AvailableNodesQuerier, NodesConverter, SubnetQuerier, SubnetQueryBy};
use futures::TryFutureExt;
use ic_base_types::NodeId;
use ic_base_types::{RegistryVersion, SubnetId};
use ic_interfaces_registry::{RegistryClient, RegistryValue, ZERO_REGISTRY_VERSION};
use ic_management_types::{
    Artifact, ArtifactReleases, Datacenter, DatacenterOwner, Guest, Network, NetworkError, Node, NodeProviderDetails, NodeProvidersResponse,
    Operator, Provider, Release, Subnet, SubnetMetadata, UpdateElectedHostosVersionsProposal, UpdateElectedReplicaVersionsProposal,
};
use ic_protobuf::registry::api_boundary_node::v1::ApiBoundaryNodeRecord;
use ic_protobuf::registry::crypto::v1::PublicKey;
use ic_protobuf::registry::hostos_version::v1::HostosVersionRecord;
use ic_protobuf::registry::replica_version::v1::{BlessedReplicaVersions, ReplicaVersionRecord};
use ic_protobuf::registry::unassigned_nodes_config::v1::UnassignedNodesConfigRecord;
use ic_protobuf::registry::{dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, subnet::v1::SubnetRecord};
use ic_registry_client::client::ThresholdSigPublicKey;
use ic_registry_client_fake::FakeRegistryClient;
use ic_registry_client_helpers::node::NodeRegistry;
use ic_registry_common_proto::pb::local_store::v1::{ChangelogEntry as PbChangelogEntry, KeyMutation as PbKeyMutation, MutationType};
use ic_registry_keys::{
    make_blessed_replica_versions_key, HOSTOS_VERSION_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX,
    REPLICA_VERSION_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_keys::{make_crypto_threshold_signing_pubkey_key, ROOT_SUBNET_ID_KEY};
use ic_registry_keys::{API_BOUNDARY_NODE_RECORD_KEY_PREFIX, DATA_CENTER_KEY_PREFIX};
use ic_registry_local_registry::LocalRegistry;
use ic_registry_local_store::{Changelog, ChangelogEntry, KeyMutation, LocalStoreImpl};
use ic_registry_nns_data_provider::registry::RegistryCanister;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use regex::Regex;
use registry_canister::mutations::common::decode_registry_value;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
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
use url::Url;
extern crate env_logger;

use anyhow::Result;
use ic_registry_client_helpers::subnet::SubnetListRegistry;

pub const NNS_SUBNET_NAME: &str = "NNS";

pub const DFINITY_DCS: &str = "zh2 mr1 bo1 sh1";

pub struct RegistryState {
    network: Network,
    local_registry: Arc<LocalRegistry>,

    version: u64,
    subnets: BTreeMap<PrincipalId, Subnet>,
    nodes: BTreeMap<PrincipalId, Node>,
    operators: BTreeMap<PrincipalId, Operator>,
    node_labels_guests: Vec<Guest>,
    known_subnets: BTreeMap<PrincipalId, String>,

    guestos_releases: ArtifactReleases,
    hostos_releases: ArtifactReleases,
    ic_repo: Option<IcRepo>,
}
pub trait RegistryEntry: RegistryValue {
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

impl RegistryEntry for ReplicaVersionRecord {
    const KEY_PREFIX: &'static str = REPLICA_VERSION_KEY_PREFIX;
}

impl RegistryEntry for HostosVersionRecord {
    const KEY_PREFIX: &'static str = HOSTOS_VERSION_KEY_PREFIX;
}

impl RegistryEntry for UnassignedNodesConfigRecord {
    const KEY_PREFIX: &'static str = "unassigned_nodes_config";
}

impl RegistryEntry for ApiBoundaryNodeRecord {
    const KEY_PREFIX: &'static str = API_BOUNDARY_NODE_RECORD_KEY_PREFIX;
}

pub trait RegistryFamilyEntries {
    fn get_family_entries<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, T>>;
    fn get_family_entries_versioned<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, (u64, T)>>;
    fn get_family_entries_of_version<T: RegistryEntry + Default>(&self, version: RegistryVersion) -> Result<BTreeMap<String, (u64, T)>>;
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
                    .map(|v| (key[prefix_length..].to_string(), T::decode(v.as_slice()).expect("invalid registry value")))
            })
            .collect::<BTreeMap<_, _>>())
    }

    fn get_family_entries_versioned<T: RegistryEntry + Default>(&self) -> Result<BTreeMap<String, (u64, T)>> {
        self.get_family_entries_of_version(self.get_latest_version())
    }

    fn get_family_entries_of_version<T: RegistryEntry + Default>(&self, version: RegistryVersion) -> Result<BTreeMap<String, (u64, T)>> {
        let prefix_length = T::KEY_PREFIX.len();
        Ok(self
            .get_key_family(T::KEY_PREFIX, version)?
            .iter()
            .filter_map(|key| {
                self.get_versioned_value(key, version)
                    .map(|r| {
                        r.value.as_ref().map(|v| {
                            (
                                key[prefix_length..].to_string(),
                                (r.version.get(), T::decode(v.as_slice()).expect("invalid registry value")),
                            )
                        })
                    })
                    .unwrap_or_else(|_| panic!("failed to get entry {} for type {}", key, std::any::type_name::<T>()))
            })
            .collect::<BTreeMap<_, _>>())
    }
}

trait ReleasesOps {
    fn get_active_branches(&self) -> Vec<String>;
}
impl ReleasesOps for ArtifactReleases {
    fn get_active_branches(&self) -> Vec<String> {
        const NUM_RELEASE_BRANCHES_TO_KEEP: usize = 2;
        if self.releases.is_empty() {
            warn!("No {} releases found in the registry. THIS MAY BE A BUG!", self.artifact);
        } else {
            info!(
                "{} versions: {}",
                self.artifact,
                self.releases
                    .iter()
                    .map(|r| format!("{} ({})", r.commit_hash.clone(), r.branch))
                    .join("\n")
            );
        };
        self.releases
            .clone()
            .into_iter()
            .rev()
            .map(|rr| rr.branch)
            .unique()
            .take(NUM_RELEASE_BRANCHES_TO_KEEP)
            .collect::<Vec<_>>()
    }
}

#[allow(dead_code)]
impl RegistryState {
    pub async fn new(network: &Network, without_update_loop: bool) -> Self {
        sync_local_store(network).await.expect("failed to init local store");

        if !without_update_loop {
            let closure_network = network.clone();
            tokio::spawn(async move {
                loop {
                    if let Err(e) = sync_local_store(&closure_network).await {
                        error!("Failed to update local registry: {}", e);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
        }

        let local_registry_path = local_registry_path(&network.clone());
        info!(
            "Using local registry path for network {}: {}",
            network.name,
            local_registry_path.display()
        );
        let local_registry: Arc<LocalRegistry> =
            Arc::new(LocalRegistry::new(local_registry_path, Duration::from_millis(1000)).expect("Failed to create local registry"));

        Self {
            network: network.clone(),
            local_registry,
            version: 0,
            subnets: BTreeMap::<PrincipalId, Subnet>::new(),
            nodes: BTreeMap::new(),
            operators: BTreeMap::new(),
            node_labels_guests: Vec::new(),
            guestos_releases: ArtifactReleases::new(Artifact::GuestOs),
            hostos_releases: ArtifactReleases::new(Artifact::HostOs),
            ic_repo: Some(IcRepo::new().expect("failed to init ic repo")),
            known_subnets: [
                (
                    "uzr34-akd3s-xrdag-3ql62-ocgoh-ld2ao-tamcv-54e7j-krwgb-2gm4z-oqe",
                    "Internet Identity, tECDSA backup",
                ),
                ("w4rem-dv5e3-widiz-wbpea-kbttk-mnzfm-tzrc7-svcj3-kbxyb-zamch-hqe", "Bitcoin"),
                ("eq6en-6jqla-fbu5s-daskr-h6hx2-376n5-iqabl-qgrng-gfqmv-n3yjr-mqe", "Open Chat 1"),
                ("2fq7c-slacv-26cgz-vzbx2-2jrcs-5edph-i5s2j-tck77-c3rlz-iobzx-mqe", "Open Chat 2"),
                ("pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae", "tECDSA signing"),
                ("x33ed-h457x-bsgyx-oqxqf-6pzwv-wkhzr-rm2j3-npodi-purzm-n66cg-gae", "SNS"),
                ("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe", "European"),
            ]
            .iter()
            .map(|(p, name)| (PrincipalId::from_str(p).expect("invalid principal id"), name.to_string()))
            .collect(),
        }
    }

    pub fn update_node_labels_guests(&mut self, node_label_guests: Vec<Guest>) {
        self.node_labels_guests = node_label_guests;
        if !self.network.is_mainnet() {
            for g in &mut self.node_labels_guests {
                g.dfinity_owned = true;
            }
        }
    }

    pub async fn update_node_details(&mut self, providers: &[NodeProviderDetails]) -> anyhow::Result<()> {
        self.local_registry.sync_with_local_store().await.map_err(|e| anyhow::anyhow!(e))?;
        self.update_releases().await?;
        self.update_operators(providers)?;
        self.update_nodes()?;
        self.update_subnets()?;
        self.version = self.local_registry.get_latest_version().get();

        Ok(())
    }

    pub async fn get_elected_guestos_versions(&self) -> Result<Vec<String>, anyhow::Error> {
        match self
            .local_registry
            .get_value(&make_blessed_replica_versions_key(), self.local_registry.get_latest_version())
        {
            Ok(Some(bytes)) => {
                let cfg = BlessedReplicaVersions::decode(&bytes[..]).expect("Error decoding BlessedReplicaVersions from the LocalRegistry");

                Ok(cfg.blessed_version_ids)
            }
            _ => Err(anyhow::anyhow!("No elected GuestOS versions found".to_string(),)),
        }
    }
    pub async fn get_elected_hostos_versions(&self) -> Result<Vec<String>, anyhow::Error> {
        let registry_version = self.local_registry.get_latest_version();
        let keys = self.local_registry.get_key_family(HOSTOS_VERSION_KEY_PREFIX, registry_version)?;

        let mut records = Vec::new();
        for key in keys {
            let bytes = self.local_registry.get_value(&key, registry_version);
            let hostos_version_proto = ic_registry_client_helpers::deserialize_registry_value::<HostosVersionRecord>(bytes)?.unwrap_or_default();
            records.push(hostos_version_proto.hostos_version_id)
        }

        Ok(records)
    }

    async fn update_releases(&mut self) -> Result<()> {
        // If the network isn't mainnet we don't need to check git branches
        if !self.network.eq(&Network::new("mainnet", &vec![]).await.unwrap()) {
            return Ok(());
        }
        if self.ic_repo.is_some() {
            lazy_static! {
                // TODO: We don't need to distinguish release branch and name, they can be the same
                static ref RELEASE_BRANCH_GROUP: &'static str = "release_branch";
                static ref RELEASE_NAME_GROUP: &'static str = "release_name";
                static ref DATETIME_NAME_GROUP: &'static str = "datetime";
                // example: rc--2021-09-13_18-32
                static ref RE: Regex = Regex::new(&format!(r#"(?P<{}>(?P<{}>rc--(?P<{}>\d{{4}}-\d{{2}}-\d{{2}}_\d{{2}}-\d{{2}}))(?P<discardable_suffix>.*))$"#,
                    *RELEASE_BRANCH_GROUP,
                    *RELEASE_NAME_GROUP,
                    *DATETIME_NAME_GROUP,
                )).unwrap();
            }
            let blessed_replica_versions = self.get_elected_guestos_versions().await?;
            let elected_hostos_versions = self.get_elected_hostos_versions().await?;

            let blessed_versions: HashSet<&String> = blessed_replica_versions.iter().chain(elected_hostos_versions.iter()).collect();

            // A HashMap from the git revision to the latest commit branch in which the
            // commit is present
            let mut commit_to_release: HashMap<String, Release> = HashMap::new();
            blessed_versions.into_iter().for_each(|commit_hash| {
                let ic_repo = self.ic_repo.as_mut().unwrap();
                match ic_repo.get_branches_with_commit(commit_hash) {
                    // For each commit get a list of branches that have the commit
                    Ok(branches) => {
                        debug!("Git rev {} ==> {} branches: {}", commit_hash, branches.len(), branches.join(", "));
                        for branch in branches.into_iter().sorted() {
                            match RE.captures(&branch) {
                                Some(capture) => {
                                    let release_branch = capture.name(&RELEASE_BRANCH_GROUP).expect("release regex misconfiguration").as_str();
                                    let release_name = capture.name(&RELEASE_NAME_GROUP).expect("release regex misconfiguration").as_str();
                                    let release_datetime = chrono::NaiveDateTime::parse_from_str(
                                        capture.name(&DATETIME_NAME_GROUP).expect("release regex misconfiguration").as_str(),
                                        "%Y-%m-%d_%H-%M",
                                    )
                                    .expect("invalid datetime format");

                                    commit_to_release.insert(
                                        commit_hash.clone(),
                                        Release {
                                            name: release_name.to_string(),
                                            branch: release_branch.to_string(),
                                            commit_hash: commit_hash.clone(),
                                            previous_patch_release: None,
                                            time: release_datetime,
                                        },
                                    );
                                    break;
                                }
                                None => {
                                    if branch != "master" && branch != "HEAD" {
                                        debug!("Git rev {}: branch {} does not match the RC regex", &commit_hash, &branch);
                                    }
                                }
                            };
                        }
                    }
                    Err(e) => error!("failed to find branches for git rev: {}; {}", &commit_hash, e),
                }
            });

            for (blessed_versions, ArtifactReleases { artifact, releases }) in [
                (blessed_replica_versions, &mut self.guestos_releases),
                (elected_hostos_versions, &mut self.hostos_releases),
            ] {
                releases.clear();
                releases.extend(
                    blessed_versions
                        .iter()
                        .map(|version| commit_to_release.get(version).unwrap().clone())
                        .sorted_by_key(|rr| rr.time)
                        .collect::<Vec<Release>>(),
                );
                debug!("Updated {} releases to {:?}", artifact, releases);
            }
        }
        Ok(())
    }

    fn update_operators(&mut self, providers: &[NodeProviderDetails]) -> Result<()> {
        let providers = providers.iter().map(|p| (p.principal_id, p)).collect::<BTreeMap<_, _>>();
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
                            let (continent, country, city): (_, _, _) = dc.region.splitn(3, ',').map(|s| s.to_string()).collect_tuple().unwrap_or((
                                "Unknown".to_string(),
                                "Unknown".to_string(),
                                "Unknown".to_string(),
                            ));

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
        self.node_labels_guests
            .iter()
            .find(|g| g.ipv6 == Ipv6Addr::from_str(&nr.http.clone().unwrap().ip_addr).unwrap())
            .cloned()
    }

    fn update_nodes(&mut self) -> Result<()> {
        let node_entries = self.local_registry.get_family_entries_versioned::<NodeRecord>()?;
        let dfinity_dcs = DFINITY_DCS.split(' ').map(|dc| dc.to_string().to_lowercase()).collect::<HashSet<_>>();
        let api_boundary_nodes: BTreeMap<String, ApiBoundaryNodeRecord> = self.local_registry.get_family_entries()?;

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
                let dc_name: String = match &operator.datacenter {
                    Some(dc) => dc.name.to_lowercase(),
                    None => String::new(),
                };
                (
                    principal,
                    Node {
                        principal,
                        dfinity_owned: Some(dfinity_dcs.contains(&dc_name) || guest.as_ref().map(|g| g.dfinity_owned).unwrap_or_default()),
                        ip_addr,
                        hostname: guest
                            .as_ref()
                            .map(|g| g.name.clone())
                            .unwrap_or_else(|| {
                                format!(
                                    "{}-{}",
                                    operator.datacenter.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| "??".to_string()),
                                    p.to_string().split_once('-').map(|(first, _)| first).unwrap_or("?????")
                                )
                            })
                            .into(),
                        subnet_id: self
                            .local_registry
                            .get_subnet_id_from_node_id(NodeId::new(principal), self.local_registry.get_latest_version())
                            .expect("failed to get subnet id")
                            .map(|s| s.get()),
                        hostos_version: nr.hostos_version_id.clone().unwrap_or_default(),
                        hostos_release: self
                            .hostos_releases
                            .releases
                            .iter()
                            .find(|r| r.commit_hash == nr.hostos_version_id.clone().unwrap_or_default())
                            .cloned(),
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
                        is_api_boundary_node: api_boundary_nodes.contains_key(p),
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
                            .guestos_releases
                            .releases
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
        let proposal_agent = proposal::ProposalAgent::new(self.network.get_nns_urls());

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

    pub async fn open_elect_replica_proposals(&self) -> Result<Vec<UpdateElectedReplicaVersionsProposal>> {
        let proposal_agent = proposal::ProposalAgent::new(self.network.get_nns_urls());
        proposal_agent.list_open_elect_replica_proposals().await
    }

    pub async fn open_elect_hostos_proposals(&self) -> Result<Vec<UpdateElectedHostosVersionsProposal>> {
        let proposal_agent = proposal::ProposalAgent::new(self.network.get_nns_urls());
        proposal_agent.list_open_elect_hostos_proposals().await
    }

    pub async fn subnets_with_proposals(&self) -> Result<BTreeMap<PrincipalId, Subnet>> {
        let subnets = self.subnets.clone();
        let proposal_agent = proposal::ProposalAgent::new(self.network.get_nns_urls());

        let topology_proposals = proposal_agent.list_open_topology_proposals().await?;

        Ok(subnets
            .into_iter()
            .map(|(subnet_id, subnet)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|p| {
                        p.subnet_id.unwrap_or_default() == subnet_id
                            || subnet
                                .nodes
                                .iter()
                                .any(|n| p.node_ids_added.contains(&n.principal) || p.node_ids_removed.contains(&n.principal))
                    })
                    .cloned();

                (subnet_id, Subnet { proposal, ..subnet })
            })
            .collect())
    }

    pub async fn retireable_versions(&self, artifact: &Artifact) -> Result<Vec<Release>> {
        match artifact {
            Artifact::HostOs => self.retireable_hostos_versions().await,
            Artifact::GuestOs => self.retireable_guestos_versions().await,
        }
    }

    pub async fn blessed_versions(&self, artifact: &Artifact) -> Result<Vec<String>> {
        match artifact {
            Artifact::HostOs => self.get_elected_hostos_versions().await,
            Artifact::GuestOs => self.get_elected_guestos_versions().await,
        }
    }

    pub async fn open_subnet_upgrade_proposals(&self) -> Result<Vec<SubnetUpdateProposal>> {
        let proposal_agent = proposal::ProposalAgent::new(self.get_nns_urls());

        proposal_agent.list_update_subnet_version_proposals().await
    }

    pub async fn open_upgrade_unassigned_nodes_proposals(&self) -> Result<Vec<UpdateUnassignedNodesProposal>> {
        let proposal_agent = proposal::ProposalAgent::new(self.get_nns_urls());

        proposal_agent.list_update_unassigned_nodes_version_proposals().await
    }

    async fn retireable_hostos_versions(&self) -> Result<Vec<Release>> {
        let active_releases = self.hostos_releases.get_active_branches();
        let hostos_versions: BTreeSet<String> = self.nodes.values().map(|s| s.hostos_version.clone()).collect();
        let versions_in_proposals: BTreeSet<String> = self
            .open_elect_hostos_proposals()
            .await?
            .iter()
            .flat_map(|p| p.versions_unelect.iter())
            .cloned()
            .collect();
        info!("Active releases: {}", active_releases.iter().join(", "));
        info!("HostOS versions in use on nodes: {}", hostos_versions.iter().join(", "));
        info!("HostOS versions in open proposals: {}", versions_in_proposals.iter().join(", "));
        Ok(self
            .hostos_releases
            .releases
            .clone()
            .into_iter()
            .filter(|rr| !active_releases.contains(&rr.branch))
            .filter(|rr| !hostos_versions.contains(&rr.commit_hash))
            .filter(|rr| !versions_in_proposals.contains(&rr.commit_hash))
            .collect())
    }

    async fn retireable_guestos_versions(&self) -> Result<Vec<Release>> {
        let active_releases = self.guestos_releases.get_active_branches();
        let subnet_versions: BTreeSet<String> = self.subnets.values().map(|s| s.replica_version.clone()).collect();
        let version_on_unassigned_nodes = self.get_unassigned_nodes_replica_version().await?;
        let versions_in_proposals: BTreeSet<String> = self
            .open_elect_replica_proposals()
            .await?
            .iter()
            .flat_map(|p| p.versions_unelect.iter())
            .cloned()
            .collect();
        info!("Active releases: {}", active_releases.iter().join(", "));
        info!("GuestOS versions in use on subnets: {}", subnet_versions.iter().join(", "));
        info!("GuestOS version on unassigned nodes: {}", version_on_unassigned_nodes);
        info!("GuestOS versions in open proposals: {}", versions_in_proposals.iter().join(", "));
        Ok(self
            .guestos_releases
            .releases
            .clone()
            .into_iter()
            .filter(|rr| !active_releases.contains(&rr.branch))
            .filter(|rr| !subnet_versions.contains(&rr.commit_hash) && rr.commit_hash != version_on_unassigned_nodes)
            .filter(|rr| !versions_in_proposals.contains(&rr.commit_hash))
            .collect())
    }

    pub async fn nns_replica_version(&self) -> Option<String> {
        Some(
            self.subnets()
                .get(&PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe").unwrap())?
                .replica_version
                .clone(),
        )
    }

    pub fn operators(&self) -> BTreeMap<PrincipalId, Operator> {
        self.operators.clone()
    }

    pub fn guests(&self) -> Vec<Guest> {
        self.node_labels_guests.clone()
    }

    pub fn missing_guests(&self) -> Vec<Guest> {
        let mut missing_guests = self
            .node_labels_guests
            .clone()
            .into_iter()
            .filter(|g| !self.nodes.iter().any(|(_, n)| n.label.clone().unwrap_or_default() == g.name))
            .collect::<Vec<_>>();
        missing_guests.sort_by_key(|g| g.name.clone());
        missing_guests.dedup_by_key(|g| g.name.clone());
        missing_guests
    }

    pub fn replica_releases(&self) -> Vec<Release> {
        self.guestos_releases.releases.clone()
    }

    pub fn get_nns_urls(&self) -> &Vec<Url> {
        self.network.get_nns_urls()
    }

    pub fn get_decentralized_nodes(&self, principals: &[PrincipalId]) -> Vec<decentralization::network::Node> {
        self.nodes()
            .values()
            .filter(|node| principals.contains(&node.principal))
            .map(decentralization::network::Node::from)
            .collect_vec()
    }

    pub async fn get_unassigned_nodes_replica_version(&self) -> Result<String, anyhow::Error> {
        let unassigned_config_key = ic_registry_keys::make_unassigned_nodes_config_record_key();

        match self
            .local_registry
            .get_value(&unassigned_config_key, self.local_registry.get_latest_version())
        {
            Ok(Some(bytes)) => {
                let cfg = UnassignedNodesConfigRecord::decode(&bytes[..]).expect("Error decoding UnassignedNodesConfigRecord from the LocalRegistry");

                Ok(cfg.replica_version)
            }
            _ => Err(anyhow::anyhow!("No GuestOS version for unassigned nodes found".to_string(),)),
        }
    }

    #[allow(dead_code)]
    pub async fn node(&self, node_id: PrincipalId) -> Node {
        self.nodes
            .iter()
            .filter(|(&id, _)| id == node_id)
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .1
            .clone()
    }
}

impl decentralization::network::TopologyManager for RegistryState {}

impl NodesConverter for RegistryState {
    fn get_nodes(&self, from: &Vec<PrincipalId>) -> std::result::Result<Vec<decentralization::network::Node>, NetworkError> {
        from.iter()
            .map(|n| {
                self.nodes()
                    .get(n)
                    .ok_or_else(|| NetworkError::NodeNotFound(*n))
                    .map(decentralization::network::Node::from)
            })
            .collect()
    }
}

#[async_trait]
impl SubnetQuerier for RegistryState {
    async fn subnet(&self, by: SubnetQueryBy) -> Result<decentralization::network::DecentralizedSubnet, NetworkError> {
        match by {
            SubnetQueryBy::SubnetId(id) => self
                .subnets
                .get(&id)
                .map(|s| decentralization::network::DecentralizedSubnet {
                    id: s.principal,
                    nodes: s.nodes.iter().map(decentralization::network::Node::from).collect(),
                    removed_nodes: Vec::new(),
                    min_nakamoto_coefficients: None,
                    comment: None,
                    run_log: Vec::new(),
                })
                .ok_or(NetworkError::SubnetNotFound(id)),
            SubnetQueryBy::NodeList(nodes) => {
                let subnets = nodes
                    .to_vec()
                    .iter()
                    .map(|n| self.nodes.get(&n.id).and_then(|n| n.subnet_id))
                    .collect::<BTreeSet<_>>();
                if subnets.len() > 1 {
                    return Err(NetworkError::IllegalRequest("nodes don't belong to the same subnet".to_string()));
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
    }
}

#[async_trait]
impl AvailableNodesQuerier for RegistryState {
    async fn available_nodes(&self) -> Result<Vec<decentralization::network::Node>, NetworkError> {
        let nodes = self
            .nodes_with_proposals()
            .await
            .map_err(|err| NetworkError::DataRequestError(err.to_string()))?
            .into_values()
            .filter(|n| n.subnet_id.is_none() && n.proposal.is_none() && n.duplicates.is_none() && !n.is_api_boundary_node)
            .collect::<Vec<_>>();

        let health_client = crate::health::HealthClient::new(self.network());
        let healths = health_client
            .nodes()
            .await
            .map_err(|err| NetworkError::DataRequestError(err.to_string()))?;
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

pub fn local_cache_path() -> PathBuf {
    match std::env::var("LOCAL_REGISTRY_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => match dirs::cache_dir() {
            Some(cache_dir) => cache_dir,
            None => PathBuf::from("/tmp"),
        },
    }
    .join("ic-registry-cache")
}

pub fn local_registry_path(network: &Network) -> PathBuf {
    local_cache_path().join(Path::new(network.name.as_str())).join("local_registry")
}

pub async fn nns_public_key(registry_canister: &RegistryCanister) -> anyhow::Result<ThresholdSigPublicKey> {
    let (nns_subnet_id_vec, _) = registry_canister
        .get_value(ROOT_SUBNET_ID_KEY.as_bytes().to_vec(), None)
        .await
        .map_err(|e| anyhow::format_err!("failed to get root subnet: {}", e))?;
    let nns_subnet_id = decode_registry_value::<ic_protobuf::types::v1::SubnetId>(nns_subnet_id_vec);
    let (nns_pub_key_vec, _) = registry_canister
        .get_value(
            make_crypto_threshold_signing_pubkey_key(SubnetId::new(PrincipalId::try_from(nns_subnet_id.principal_id.unwrap().raw).unwrap()))
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

/// Sync all versions of the registry, up to the latest one.
pub async fn sync_local_store(target_network: &Network) -> anyhow::Result<()> {
    let local_registry_path = local_registry_path(target_network);
    let local_store = Arc::new(LocalStoreImpl::new(local_registry_path.clone()));
    let nns_urls = target_network.get_nns_urls().clone();
    let registry_canister = RegistryCanister::new(nns_urls);
    let mut local_latest_version = if !Path::new(&local_registry_path).exists() {
        ZERO_REGISTRY_VERSION
    } else {
        let registry_cache = FakeRegistryClient::new(local_store.clone());
        registry_cache.update_to_latest_version();
        registry_cache.get_latest_version()
    };
    let mut updates = vec![];
    let nns_public_key = nns_public_key(&registry_canister).await?;

    loop {
        match registry_canister.get_latest_version().await {
            Ok(remote_version) => match local_latest_version.get().cmp(&remote_version) {
                Ordering::Less => {
                    info!("Registry version local {} < remote {}", local_latest_version.get(), remote_version);
                }
                Ordering::Equal => {
                    debug!("Local Registry version {} is up to date", local_latest_version.get());
                    break;
                }
                Ordering::Greater => {
                    warn!(
                        "Removing faulty local copy of the registry for the IC network {}: {}",
                        target_network.name,
                        local_registry_path.display()
                    );
                    std::fs::remove_dir_all(&local_registry_path)?;
                    panic!(
                        "Registry version local {} > remote {}, this should never happen",
                        local_latest_version, remote_version
                    );
                }
            },
            Err(e) => {
                error!("Failed to get latest registry version: {}", e);
            }
        }
        if let Ok((mut initial_records, _, _)) = registry_canister
            .get_certified_changes_since(local_latest_version.get(), &nns_public_key)
            .await
        {
            initial_records.sort_by_key(|tr| tr.version);
            let changelog = initial_records.iter().fold(Changelog::default(), |mut cl, r| {
                let rel_version = (r.version - local_latest_version).get();
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
                let v = RegistryVersion::from(i as u64 + 1 + local_latest_version.get());
                let local_registry_path = local_registry_path.clone();
                updates.push(async move {
                    let path_str = format!("{:016x}.pb", v.get());
                    // 00 01 02 03 04 / 05 / 06 / 07.pb
                    let v_path = &[&path_str[0..10], &path_str[10..12], &path_str[12..14], &path_str[14..19]]
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

            local_latest_version = local_latest_version.add(RegistryVersion::new(versions_count as u64));

            debug!("Sync reached version {local_latest_version}");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    futures::future::join_all(updates).await;
    Ok(())
}

pub async fn poll(registry_state: Arc<RwLock<RegistryState>>, target_network: Network) {
    let nns_urls = target_network.get_nns_urls().clone();
    let registry_canister = RegistryCanister::new(nns_urls);
    loop {
        sleep(Duration::from_secs(1)).await;
        let latest_version = if let Ok(v) = registry_canister.get_latest_version().await {
            v
        } else {
            continue;
        };
        if latest_version != registry_state.read().await.version() {
            fetch_and_add_node_labels_guests_to_registry(&target_network, &registry_state).await;
            update_node_details(&registry_state).await;
        } else {
            debug!(
                "Skipping update. Registry already on latest version: {}",
                registry_state.read().await.version()
            )
        }
    }
}

// TODO: try to get rid of node_labels data source
async fn fetch_and_add_node_labels_guests_to_registry(target_network: &Network, registry_state: &Arc<RwLock<RegistryState>>) {
    let guests_result = node_labels::query_guests(&target_network.name).await;

    match guests_result {
        Ok(node_labels_guests) => {
            let mut registry_state = registry_state.write().await;
            registry_state.update_node_labels_guests(node_labels_guests);
        }
        Err(e) => {
            warn!("Failed querying guests file: {}", e);
        }
    }
}

pub async fn update_node_details(registry_state: &Arc<RwLock<RegistryState>>) {
    let network = registry_state.read().await.network();
    match query_ic_dashboard_list::<NodeProvidersResponse>(&network, "v3/node-providers").await {
        Ok(node_providers_response) => {
            let mut registry_state = registry_state.write().await;
            let update = registry_state.update_node_details(&node_providers_response.node_providers).await;
            if let Err(e) = update {
                warn!("failed state update: {}", e);
            }
        }
        Err(e) => {
            warn!("Failed querying IC dashboard {}", e);
        }
    }
}
