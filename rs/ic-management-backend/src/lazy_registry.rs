use indexmap::{IndexMap, IndexSet};
use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use decentralization::network::{AvailableNodesQuerier, DecentralizedSubnet, NodesConverter, SubnetQuerier, SubnetQueryBy};
use futures::future::BoxFuture;
use ic_interfaces_registry::RegistryClient;
use ic_interfaces_registry::{RegistryClientVersionedResult, RegistryValue};
use ic_management_types::{
    Datacenter, DatacenterOwner, Guest, Network, NetworkError, Node, NodeProvidersResponse, Operator, Provider, Subnet, SubnetMetadata,
};
use ic_protobuf::registry::firewall::v1::FirewallRuleSet;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord, dc::v1::DataCenterRecord, hostos_version::v1::HostosVersionRecord,
    replica_version::v1::ReplicaVersionRecord, subnet::v1::SubnetRecord, unassigned_nodes_config::v1::UnassignedNodesConfigRecord,
};
use ic_registry_client_helpers::node::NodeRegistry;
use ic_registry_client_helpers::{node::NodeRecord, node_operator::NodeOperatorRecord};
use ic_registry_keys::{
    make_firewall_rules_record_key, FirewallRulesScope, API_BOUNDARY_NODE_RECORD_KEY_PREFIX, DATA_CENTER_KEY_PREFIX, HOSTOS_VERSION_KEY_PREFIX,
    NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, NODE_REWARDS_TABLE_KEY, REPLICA_VERSION_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_local_registry::LocalRegistry;
use ic_registry_subnet_type::SubnetType;
use ic_types::{NodeId, PrincipalId, RegistryVersion};
use itertools::Itertools;
use log::{debug, warn};
use mockall::mock;
use tokio::sync::RwLock;
use tokio::try_join;

use crate::health::HealthStatusQuerier;
use crate::node_labels;
use crate::proposal::ProposalAgent;
use crate::public_dashboard::query_ic_dashboard_list;
use crate::registry::{DFINITY_DCS, NNS_SUBNET_NAME};

const KNOWN_SUBNETS: &[(&str, &str)] = &[
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
];

pub trait LazyRegistry:
    LazyRegistryFamilyEntries + NodesConverter + SubnetQuerier + decentralization::network::TopologyManager + AvailableNodesQuerier + Send + Sync
{
    fn node_labels(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<Guest>>>>;

    fn elected_guestos(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<String>>>>;

    fn elected_hostos(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<String>>>>;

    fn sync_with_nns(&self) -> BoxFuture<'_, anyhow::Result<()>>;

    fn operators(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Operator>>>>;

    fn nodes(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Node>>>>;

    fn firewall_rule_set(&self, firewall_rule_scope: FirewallRulesScope) -> BoxFuture<'_, anyhow::Result<FirewallRuleSet>>;

    fn subnets(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Subnet>>>>;

    fn nodes_and_proposals(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Node>>>>;

    fn nns_replica_version(&self) -> BoxFuture<'_, anyhow::Result<Option<String>>> {
        Box::pin(async {
            Ok(self.subnets().await?.values().find_map(|s| {
                if s.subnet_type.eq(&SubnetType::System) {
                    return Some(s.replica_version.clone());
                }
                None
            }))
        })
    }

    fn missing_guests(&self) -> BoxFuture<'_, anyhow::Result<Vec<Guest>>> {
        Box::pin(async {
            let nodes = self.nodes().await?;
            let mut missing_guests = self
                .node_labels()
                .await?
                .iter()
                .filter(|g| !nodes.iter().any(|(_, n)| n.label.clone().unwrap_or_default() == g.name))
                .cloned()
                .collect_vec();

            missing_guests.sort_by_key(|g| g.name.to_owned());
            missing_guests.dedup_by_key(|g| g.name.to_owned());

            Ok(missing_guests)
        })
    }

    fn get_nodes_from_ids<'a>(&'a self, principals: &'a [PrincipalId]) -> BoxFuture<'a, anyhow::Result<Vec<Node>>> {
        Box::pin(async {
            let all_nodes = self.nodes().await?;
            Ok(principals.iter().filter_map(|p| all_nodes.get(p).cloned()).collect())
        })
    }

    fn unassigned_nodes_replica_version(&self) -> BoxFuture<'_, anyhow::Result<Arc<String>>>;

    fn get_api_boundary_nodes(&self) -> anyhow::Result<Vec<(String, ApiBoundaryNodeRecord)>>;

    fn get_node_rewards_table(&self) -> anyhow::Result<IndexMap<String, NodeRewardsTable>>;

    fn get_unassigned_nodes(&self) -> anyhow::Result<Option<UnassignedNodesConfigRecord>>;

    fn get_datacenters(&self) -> anyhow::Result<Vec<DataCenterRecord>>;

    fn elected_guestos_records(&self) -> anyhow::Result<Vec<ReplicaVersionRecord>>;

    fn elected_hostos_records(&self) -> anyhow::Result<Vec<HostosVersionRecord>>;

    fn update_proposal_data(&self) -> BoxFuture<'_, anyhow::Result<()>>;

    fn subnets_and_proposals(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Subnet>>>> {
        Box::pin(async {
            let subnets = self.subnets().await?;

            self.update_proposal_data().await?;

            if subnets.iter().any(|(_, s)| s.proposal.is_some()) {
                return Ok(subnets);
            }

            self.subnets().await
        })
    }
}

impl NodesConverter for Box<dyn LazyRegistry> {
    fn get_nodes<'a>(&'a self, node_ids: &'a [PrincipalId]) -> BoxFuture<'a, Result<Vec<Node>, ic_management_types::NetworkError>> {
        Box::pin(async {
            let nodes = self
                .nodes()
                .await
                .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?;
            node_ids
                .iter()
                .map(|n| nodes.get(n).cloned().ok_or(ic_management_types::NetworkError::NodeNotFound(*n)))
                .collect()
        })
    }
}

pub struct LazyRegistryImpl
where
    Self: Send + Sync,
{
    local_registry: LocalRegistry,
    network: Network,

    subnets: RwLock<Option<Arc<IndexMap<PrincipalId, Subnet>>>>,
    nodes: RwLock<Option<Arc<IndexMap<PrincipalId, Node>>>>,
    operators: RwLock<Option<Arc<IndexMap<PrincipalId, Operator>>>>,
    node_labels_guests: RwLock<Option<Arc<Vec<Guest>>>>,
    elected_guestos: RwLock<Option<Arc<Vec<String>>>>,
    elected_hostos: RwLock<Option<Arc<Vec<String>>>>,
    unassigned_nodes_replica_version: RwLock<Option<Arc<String>>>,
    firewall_rule_set: RwLock<Option<Arc<IndexMap<String, FirewallRuleSet>>>>,
    offline: bool,
    proposal_agent: Arc<dyn ProposalAgent>,
    guest_labels_cache_path: PathBuf,
    health_client: Arc<dyn HealthStatusQuerier>,
    version_height: Option<u64>,
}

pub trait LazyRegistryEntry: RegistryValue {
    const KEY_PREFIX: &'static str;
}

impl LazyRegistryEntry for DataCenterRecord {
    const KEY_PREFIX: &'static str = DATA_CENTER_KEY_PREFIX;
}

impl LazyRegistryEntry for NodeOperatorRecord {
    const KEY_PREFIX: &'static str = NODE_OPERATOR_RECORD_KEY_PREFIX;
}

impl LazyRegistryEntry for NodeRecord {
    const KEY_PREFIX: &'static str = NODE_RECORD_KEY_PREFIX;
}

impl LazyRegistryEntry for SubnetRecord {
    const KEY_PREFIX: &'static str = SUBNET_RECORD_KEY_PREFIX;
}

impl LazyRegistryEntry for ReplicaVersionRecord {
    const KEY_PREFIX: &'static str = REPLICA_VERSION_KEY_PREFIX;
}

impl LazyRegistryEntry for HostosVersionRecord {
    const KEY_PREFIX: &'static str = HOSTOS_VERSION_KEY_PREFIX;
}

impl LazyRegistryEntry for UnassignedNodesConfigRecord {
    const KEY_PREFIX: &'static str = "unassigned_nodes_config";
}

impl LazyRegistryEntry for ApiBoundaryNodeRecord {
    const KEY_PREFIX: &'static str = API_BOUNDARY_NODE_RECORD_KEY_PREFIX;
}

impl LazyRegistryEntry for BlessedReplicaVersions {
    const KEY_PREFIX: &'static str = "blessed_replica_versions";
}

impl LazyRegistryEntry for NodeRewardsTable {
    const KEY_PREFIX: &'static str = NODE_REWARDS_TABLE_KEY;
}

pub trait LazyRegistryFamilyEntries {
    fn get_key_family(&self, key_prefix: &str, version: RegistryVersion) -> anyhow::Result<Vec<String>>;
    fn get_versioned_value(&self, key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>>;
    fn get_latest_version(&self) -> RegistryVersion;
}

fn get_family_entries<T: LazyRegistryEntry + Default>(reg: &impl LazyRegistryFamilyEntries) -> anyhow::Result<IndexMap<String, T>> {
    let family = get_family_entries_versioned::<T>(reg)?;
    Ok(family.into_iter().map(|(k, (_, v))| (k, v)).collect())
}
fn get_family_entries_versioned<T: LazyRegistryEntry + Default>(reg: &impl LazyRegistryFamilyEntries) -> anyhow::Result<IndexMap<String, (u64, T)>> {
    get_family_entries_of_version(reg, reg.get_latest_version())
}
fn get_family_entries_of_version<T: LazyRegistryEntry + Default>(
    reg: &impl LazyRegistryFamilyEntries,
    version: RegistryVersion,
) -> anyhow::Result<IndexMap<String, (u64, T)>> {
    let prefix_length = T::KEY_PREFIX.len();
    Ok(reg
        .get_key_family(T::KEY_PREFIX, version)?
        .iter()
        .filter_map(|key| {
            let r = reg
                .get_versioned_value(key, version)
                .unwrap_or_else(|_| panic!("Failed to get entry {} for type {}", key, std::any::type_name::<T>()));
            r.as_ref().map(|v| {
                (
                    key[prefix_length..].to_string(),
                    (r.version.get(), T::decode(v.as_slice()).expect("Invalid registry value")),
                )
            })
        })
        .collect())
}

impl LazyRegistryFamilyEntries for LazyRegistryImpl {
    fn get_key_family(&self, key_prefix: &str, version: RegistryVersion) -> anyhow::Result<Vec<String>> {
        Ok(self.local_registry.get_key_family(key_prefix, version)?)
    }

    fn get_versioned_value(&self, key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>> {
        self.local_registry.get_versioned_value(key, version)
    }

    fn get_latest_version(&self) -> RegistryVersion {
        self.version_height
            .map(RegistryVersion::new)
            .unwrap_or_else(|| self.local_registry.get_latest_version())
    }
}

impl LazyRegistryImpl {
    pub fn new(
        local_registry: LocalRegistry,
        network: Network,
        offline: bool,
        proposal_agent: Arc<dyn ProposalAgent>,
        guest_labels_cache_path: PathBuf,
        health_client: Arc<dyn HealthStatusQuerier>,
        version_height: Option<u64>,
    ) -> Self {
        Self {
            local_registry,
            network,
            subnets: RwLock::new(None),
            nodes: RwLock::new(None),
            operators: RwLock::new(None),
            node_labels_guests: RwLock::new(None),
            elected_guestos: RwLock::new(None),
            elected_hostos: RwLock::new(None),
            unassigned_nodes_replica_version: RwLock::new(None),
            firewall_rule_set: RwLock::new(None),
            offline,
            proposal_agent,
            guest_labels_cache_path,
            health_client,
            version_height,
        }
    }

    fn node_record_guest(guests: Arc<Vec<Guest>>, nr: &NodeRecord) -> Option<Guest> {
        guests
            .iter()
            .find(|g| g.ipv6 == Ipv6Addr::from_str(&nr.http.clone().unwrap().ip_addr).unwrap())
            .cloned()
    }

    fn node_ip_addr(nr: &NodeRecord) -> Ipv6Addr {
        Ipv6Addr::from_str(&nr.http.clone().expect("missing ipv6 address").ip_addr).expect("invalid ipv6 address")
    }
}

impl LazyRegistry for LazyRegistryImpl {
    // See if making it async would change anything
    fn node_labels(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<Guest>>>> {
        Box::pin(async {
            if let Some(guests) = self.node_labels_guests.read().await.as_ref() {
                return Ok(guests.to_owned());
            }

            if !self.network.is_mainnet() && !self.network.eq(&Network::staging_unchecked().unwrap()) {
                let res = Arc::new(vec![]);
                *self.node_labels_guests.write().await = Some(res.clone());
                return Ok(res);
            }

            let guests = match node_labels::query_guests(&self.network.name, Some(self.guest_labels_cache_path.clone()), self.offline).await {
                Ok(g) => g,
                Err(e) => {
                    warn!("Failed to query node labels: {}", e);
                    vec![]
                }
            };

            let guests = Arc::new(guests);
            *self.node_labels_guests.write().await = Some(guests.clone());
            Ok(guests)
        })
    }

    fn elected_guestos(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<String>>>> {
        Box::pin(async {
            if let Some(elected) = self.elected_guestos.read().await.as_ref() {
                return Ok(elected.to_owned());
            }

            let record = get_family_entries::<BlessedReplicaVersions>(self)?
                .first_entry()
                .ok_or(anyhow::anyhow!("No blessed replica versions found"))?
                .get()
                .to_owned();

            let record = Arc::new(record.blessed_version_ids);
            *self.elected_guestos.write().await = Some(record.clone());
            Ok(record)
        })
    }

    fn elected_guestos_records(&self) -> anyhow::Result<Vec<ReplicaVersionRecord>> {
        Ok(get_family_entries_versioned::<ReplicaVersionRecord>(self)
            .map_err(|e| anyhow::anyhow!("Couldn't get elected versions: {:?}", e))?
            .into_iter()
            .map(|(_, (_, record))| record)
            .collect())
    }

    fn elected_hostos(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<String>>>> {
        Box::pin(async {
            if let Some(elected) = self.elected_hostos.read().await.as_ref() {
                return Ok(elected.to_owned());
            }

            let record = get_family_entries::<HostosVersionRecord>(self)?
                .values()
                .map(|v| v.hostos_version_id.to_owned())
                .collect_vec();

            let record = Arc::new(record);
            *self.elected_hostos.write().await = Some(record.clone());
            Ok(record)
        })
    }

    fn elected_hostos_records(&self) -> anyhow::Result<Vec<HostosVersionRecord>> {
        Ok(get_family_entries_versioned::<HostosVersionRecord>(self)
            .map_err(|e| anyhow::anyhow!("Couldn't get elected versions: {:?}", e))?
            .into_iter()
            .map(|(_, (_, record))| record)
            .collect())
    }

    // Resets the whole state of after fetching so that targets can be
    // recalculated
    fn sync_with_nns(&self) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async {
            self.local_registry.sync_with_nns().await.map_err(|e| anyhow::anyhow!(e))?;
            *self.subnets.write().await = None;
            *self.nodes.write().await = None;
            *self.operators.write().await = None;

            *self.elected_guestos.write().await = None;
            *self.elected_hostos.write().await = None;
            *self.unassigned_nodes_replica_version.write().await = None;
            *self.firewall_rule_set.write().await = None;

            Ok(())
        })
    }

    fn operators(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Operator>>>> {
        Box::pin(async {
            if let Some(operators) = self.operators.read().await.as_ref() {
                return Ok(operators.to_owned());
            }

            // Fetch node providers
            let node_providers = match self.offline {
                false => query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers").await?,
                true => NodeProvidersResponse { node_providers: vec![] },
            };
            let node_providers: IndexMap<_, _> = node_providers.node_providers.iter().map(|p| (p.principal_id, p)).collect();
            let data_centers = get_family_entries::<DataCenterRecord>(self)?;
            let operators = get_family_entries::<NodeOperatorRecord>(self)?;

            let records: IndexMap<_, _> = operators
                .iter()
                .map(|(p, or)| {
                    let operator_principal = PrincipalId::from_str(p).expect("Invalid operator principal id");
                    let provider_principal = PrincipalId::try_from(or.node_provider_principal_id.as_slice());
                    (
                        operator_principal,
                        Operator {
                            principal: operator_principal,
                            provider: provider_principal
                                .map(|principal| {
                                    let mut provider = Provider {
                                        principal,
                                        name: None,
                                        website: None,
                                    };
                                    if let Some(dashboard_provider) = node_providers.get(&principal) {
                                        provider.website = dashboard_provider.website.clone();
                                        provider.name = Some(dashboard_provider.display_name.clone());
                                    } else {
                                        debug!("Node provider not found for operator: {}", operator_principal);
                                    }
                                    provider
                                })
                                .unwrap(),
                            node_allowance: or.node_allowance,
                            datacenter: data_centers.get(&or.dc_id).map(|dc| {
                                let (continent, country, area): (_, _, _) = dc
                                    .region
                                    .splitn(3, ',')
                                    .map(|s| s.to_string())
                                    .collect_tuple()
                                    .unwrap_or(("Unknown".to_string(), "Unknown".to_string(), "Unknown".to_string()));

                                Datacenter {
                                    name: dc.id.clone(),
                                    area,
                                    country,
                                    continent,
                                    owner: DatacenterOwner { name: dc.owner.clone() },
                                    latitude: dc.gps.map(|l| l.latitude as f64),
                                    longitude: dc.gps.map(|l| l.longitude as f64),
                                }
                            }),
                            rewardable_nodes: or.rewardable_nodes.iter().map(|(k, v)| (k.clone(), *v)).collect(),
                            ipv6: or.ipv6().to_string(),
                        },
                    )
                })
                .collect();

            let records = Arc::new(records);
            *self.operators.write().await = Some(records.clone());
            Ok(records)
        })
    }

    fn nodes(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Node>>>> {
        Box::pin(async {
            if let Some(nodes) = self.nodes.read().await.as_ref() {
                return Ok(nodes.to_owned());
            }

            let node_entries = get_family_entries::<NodeRecord>(self)?;
            let versioned_node_entries = get_family_entries_versioned::<NodeRecord>(self)?;
            let dfinity_dcs = DFINITY_DCS.split(' ').map(|dc| dc.to_string().to_lowercase()).collect::<IndexSet<_>>();
            let api_boundary_nodes = get_family_entries::<ApiBoundaryNodeRecord>(self)?;
            let guests = self.node_labels().await?;
            let operators = self.operators().await?;
            let nodes: IndexMap<_, _> = node_entries
                .iter()
                .map(|(p, nr)| {
                    let guest = Self::node_record_guest(guests.clone(), nr);
                    let operator = operators
                        .iter()
                        .find(|(op, _)| op.to_vec() == nr.node_operator_id)
                        .map(|(_, o)| o.to_owned());
                    if operator.is_none() && self.network.is_mainnet() {
                        warn!("Operator should not be none on mainnet for the latest height")
                    }

                    let principal = PrincipalId::from_str(p).expect("Invalid node principal id");
                    let ip_addr = Self::node_ip_addr(nr);
                    let dc_name = operator
                        .clone()
                        .map(|op| match op.datacenter {
                            Some(dc) => dc.name.to_lowercase(),
                            None => "".to_string(),
                        })
                        .unwrap_or_default();
                    (
                        principal,
                        Node {
                            principal,
                            dfinity_owned: Some(dfinity_dcs.contains(&dc_name) || guest.as_ref().map(|g| g.dfinity_owned).unwrap_or_default()),
                            ip_addr: Some(ip_addr),
                            hostname: guest
                                .as_ref()
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|| {
                                    format!(
                                        "{}-{}",
                                        operator
                                            .clone()
                                            .map(|operator| operator.datacenter.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| "??".to_string()))
                                            .unwrap_or_default(),
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
                            // TODO: map hostos release
                            hostos_release: None,
                            operator: operator.clone().unwrap_or_default(),
                            cached_features: OnceLock::new(),
                            proposal: None,
                            label: guest.map(|g| g.name),
                            duplicates: versioned_node_entries
                                .iter()
                                .filter(|(_, (_, nr2))| Self::node_ip_addr(nr2) == Self::node_ip_addr(nr))
                                .max_by_key(|(_, (version, _))| version)
                                .and_then(|(p2, _)| {
                                    if p2 == p {
                                        None
                                    } else {
                                        Some(PrincipalId::from_str(p2).expect("invalid node principal id"))
                                    }
                                }),
                            is_api_boundary_node: api_boundary_nodes.contains_key(p),
                            chip_id: nr.chip_id.clone(),
                            public_ipv4_config: nr.public_ipv4_config.clone(),
                        },
                    )
                })
                .collect();

            let nodes = Arc::new(nodes);
            *self.nodes.write().await = Some(nodes.clone());
            Ok(nodes)
        })
    }

    fn firewall_rule_set(&self, firewall_rule_scope: FirewallRulesScope) -> BoxFuture<'_, anyhow::Result<FirewallRuleSet>> {
        Box::pin(async move {
            let key = make_firewall_rules_record_key(&firewall_rule_scope);
            if let Some(firewall_rule_set) = self.firewall_rule_set.read().await.as_ref() {
                if let Some(entry) = firewall_rule_set.get(&key) {
                    return Ok(entry.to_owned());
                }
            }

            let value = match self
                .local_registry
                .get_value(&key, self.get_latest_version())
                .map_err(|e| anyhow::anyhow!(e))?
            {
                Some(v) => FirewallRuleSet::decode(v.as_slice())?,
                None => FirewallRuleSet::default(),
            };

            let mut opt_arc_map = self.firewall_rule_set.write().await;
            if let Some(arc_map) = opt_arc_map.as_mut() {
                let bag = Arc::make_mut(arc_map);
                bag.insert(key.to_owned(), value.clone());
            } else {
                let mut all = IndexMap::new();
                all.insert(key.to_owned(), value.clone());
                *opt_arc_map = Some(Arc::new(all));
            }

            Ok(value)
        })
    }

    fn subnets(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Subnet>>>> {
        Box::pin(async {
            if let Some(subnets) = self.subnets.read().await.as_ref() {
                return Ok(subnets.to_owned());
            }

            let all_nodes = self.nodes().await?;

            let subnets: IndexMap<_, _> = get_family_entries::<SubnetRecord>(self)?
                .iter()
                .enumerate()
                .map(|(i, (p, sr))| {
                    let principal = PrincipalId::from_str(p).expect("Invalid subnet principal id");
                    let subnet_nodes = all_nodes
                        .iter()
                        .filter(|(_, n)| (n.subnet_id == Some(principal)))
                        .map(|(_, n)| n)
                        .cloned()
                        .collect_vec();

                    let subnet_type = SubnetType::try_from(sr.subnet_type).unwrap();
                    (
                        principal,
                        Subnet {
                            nodes: subnet_nodes,
                            principal,
                            subnet_type,
                            metadata: SubnetMetadata {
                                name: if let Some((_, val)) = KNOWN_SUBNETS.iter().find(|(key, _)| key == p) {
                                    val.to_string()
                                } else if i == 0 {
                                    NNS_SUBNET_NAME.to_string()
                                } else {
                                    format!(
                                        "{} {}",
                                        match subnet_type {
                                            SubnetType::System => "System",
                                            SubnetType::Application | SubnetType::VerifiedApplication => "App",
                                        },
                                        i
                                    )
                                },
                                ..Default::default()
                            },
                            replica_version: sr.replica_version_id.to_owned(),
                            // TODO: map replica release
                            replica_release: None,
                            proposal: None,
                            max_ingress_bytes_per_message: sr.max_ingress_bytes_per_message,
                            max_ingress_messages_per_block: sr.max_ingress_messages_per_block,
                            max_block_payload_size: sr.max_block_payload_size,
                            unit_delay_millis: sr.unit_delay_millis,
                            initial_notary_delay_millis: sr.initial_notary_delay_millis,
                            dkg_interval_length: sr.dkg_interval_length,
                            start_as_nns: sr.start_as_nns,
                            features: sr.features,
                            max_number_of_canisters: sr.max_number_of_canisters,
                            ssh_readonly_access: sr.ssh_readonly_access.clone(),
                            ssh_backup_access: sr.ssh_backup_access.clone(),
                            ecdsa_config: sr.ecdsa_config.clone(),
                            dkg_dealings_per_block: sr.dkg_dealings_per_block,
                            is_halted: sr.is_halted,
                            halt_at_cup_height: sr.halt_at_cup_height,
                            chain_key_config: sr.chain_key_config.clone(),
                        },
                    )
                })
                .filter(|(_, s)| !s.nodes.is_empty())
                .collect();

            let subnets = Arc::new(subnets);
            *self.subnets.write().await = Some(subnets.clone());
            Ok(subnets)
        })
    }

    fn nodes_and_proposals(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Node>>>> {
        Box::pin(async {
            let nodes = self.nodes().await?;
            if nodes.iter().any(|(_, n)| n.proposal.is_some()) {
                return Ok(nodes);
            }

            self.update_proposal_data().await?;
            self.nodes().await
        })
    }

    fn update_proposal_data(&self) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async {
            if self.offline {
                return Ok(());
            }

            let nodes = self.nodes().await?;
            let subnets = self.subnets().await?;

            let topology_proposals = self.proposal_agent.list_open_topology_proposals().await?;
            let nodes: IndexMap<_, _> = nodes
                .iter()
                .map(|(p, n)| {
                    let proposal = topology_proposals
                        .iter()
                        .find(|p| p.node_ids_added.contains(&n.principal) || p.node_ids_removed.contains(&n.principal))
                        .cloned();

                    (*p, Node { proposal, ..n.clone() })
                })
                .collect();

            let subnets: IndexMap<_, _> = subnets
                .iter()
                .map(|(p, s)| {
                    let proposal = topology_proposals
                        .iter()
                        .find(|pr| {
                            pr.subnet_id.unwrap_or_default() == *p
                                || s.nodes
                                    .iter()
                                    .any(|n| pr.node_ids_added.contains(&n.principal) || pr.node_ids_removed.contains(&n.principal))
                        })
                        .cloned();

                    (*p, Subnet { proposal, ..s.clone() })
                })
                .collect();

            *self.nodes.write().await = Some(Arc::new(nodes));
            *self.subnets.write().await = Some(Arc::new(subnets));

            Ok(())
        })
    }

    fn missing_guests(&self) -> BoxFuture<'_, anyhow::Result<Vec<Guest>>> {
        Box::pin(async {
            let nodes = self.nodes().await?;
            let mut missing_guests = self
                .node_labels()
                .await?
                .iter()
                .filter(|g| !nodes.iter().any(|(_, n)| n.label.clone().unwrap_or_default() == g.name))
                .cloned()
                .collect_vec();

            missing_guests.sort_by_key(|g| g.name.to_owned());
            missing_guests.dedup_by_key(|g| g.name.to_owned());

            Ok(missing_guests)
        })
    }

    fn get_nodes_from_ids<'a>(&'a self, principals: &'a [PrincipalId]) -> BoxFuture<'a, anyhow::Result<Vec<Node>>> {
        Box::pin(async {
            Ok(self
                .nodes()
                .await?
                .values()
                .filter(|n| principals.contains(&n.principal))
                .cloned()
                .collect_vec())
        })
    }

    fn unassigned_nodes_replica_version(&self) -> BoxFuture<'_, anyhow::Result<Arc<String>>> {
        Box::pin(async {
            if let Some(v) = self.unassigned_nodes_replica_version.read().await.as_ref() {
                return Ok(v.to_owned());
            }

            let version = get_family_entries::<UnassignedNodesConfigRecord>(self)?
                .first_entry()
                .map(|v| v.get().to_owned())
                .ok_or(anyhow::anyhow!("No unassigned nodes version"))?;

            let version = Arc::new(version.replica_version);
            *self.unassigned_nodes_replica_version.write().await = Some(version.clone());
            Ok(version)
        })
    }

    fn get_api_boundary_nodes(&self) -> anyhow::Result<Vec<(String, ApiBoundaryNodeRecord)>> {
        Ok(get_family_entries_versioned::<ApiBoundaryNodeRecord>(self)
            .map_err(|e| anyhow::anyhow!("Couldn't get api boundary nodes: {:?}", e))?
            .into_iter()
            .map(|(k, (_, r))| (k, r))
            .collect_vec())
    }

    fn get_node_rewards_table(&self) -> anyhow::Result<IndexMap<String, NodeRewardsTable>> {
        get_family_entries::<NodeRewardsTable>(self).map_err(|e| anyhow::anyhow!("Couldn't get node rewards table: {:?}", e))
    }

    fn get_unassigned_nodes(&self) -> anyhow::Result<Option<UnassignedNodesConfigRecord>> {
        Ok(get_family_entries_versioned::<UnassignedNodesConfigRecord>(self)
            .map_err(|e| anyhow::anyhow!("Couldn't get unassigned nodes config: {:?}", e))?
            .into_iter()
            .map(|(_, (_, record))| record)
            .next())
    }

    fn get_datacenters(&self) -> anyhow::Result<Vec<DataCenterRecord>> {
        Ok(get_family_entries_versioned::<DataCenterRecord>(self)
            .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?
            .into_iter()
            .map(|(_, (_, record))| record)
            .collect())
    }
}

impl NodesConverter for LazyRegistryImpl {
    fn get_nodes<'a>(&'a self, from: &'a [PrincipalId]) -> BoxFuture<'a, Result<Vec<Node>, ic_management_types::NetworkError>> {
        Box::pin(async {
            let nodes = self
                .nodes()
                .await
                .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?;
            from.iter()
                .map(|n| nodes.get(n).cloned().ok_or(ic_management_types::NetworkError::NodeNotFound(*n)))
                .collect()
        })
    }
}

impl SubnetQuerier for LazyRegistryImpl {
    fn subnet(&self, by: SubnetQueryBy) -> BoxFuture<'_, Result<DecentralizedSubnet, ic_management_types::NetworkError>> {
        Box::pin(async {
            match by {
                SubnetQueryBy::SubnetId(id) => self
                    .subnets()
                    .await
                    .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?
                    .get(&id)
                    .map(|s| DecentralizedSubnet {
                        id: s.principal,
                        nodes: s.nodes.clone(),
                        added_nodes: vec![],
                        removed_nodes: vec![],
                        comment: None,
                        run_log: vec![],
                    })
                    .ok_or(ic_management_types::NetworkError::SubnetNotFound(id)),
                SubnetQueryBy::NodeList(nodes) => {
                    let reg_nodes = self.nodes().await.map_err(|e| NetworkError::DataRequestError(e.to_string()))?;
                    let subnets = nodes
                        .iter()
                        .map(|n| reg_nodes.get(&n.principal).and_then(|n| n.subnet_id))
                        .collect::<IndexSet<_>>();
                    if subnets.len() > 1 {
                        return Err(NetworkError::IllegalRequest("Nodes don't belong to the same subnet".to_owned()));
                    }
                    if let Some(Some(subnet)) = subnets.first() {
                        Ok(decentralization::network::DecentralizedSubnet {
                            id: *subnet,
                            nodes: self
                                .subnets()
                                .await
                                .map_err(|e| NetworkError::IllegalRequest(e.to_string()))?
                                .get(subnet)
                                .ok_or(NetworkError::SubnetNotFound(*subnet))?
                                .nodes
                                .to_vec(),
                            added_nodes: vec![],
                            removed_nodes: vec![],
                            comment: None,
                            run_log: vec![],
                        })
                    } else {
                        Err(NetworkError::IllegalRequest("no subnets found".to_string()))
                    }
                }
            }
        })
    }
}
impl decentralization::network::TopologyManager for LazyRegistryImpl {}

impl AvailableNodesQuerier for LazyRegistryImpl {
    fn available_nodes(&self) -> BoxFuture<'_, Result<Vec<Node>, ic_management_types::NetworkError>> {
        Box::pin(async {
            let (nodes_and_proposals, healths) = try_join!(self.nodes_and_proposals(), self.health_client.nodes())
                .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?;
            let available_nodes = nodes_and_proposals
                .values()
                .filter(|n| n.subnet_id.is_none() && n.proposal.is_none() && n.duplicates.is_none() && !n.is_api_boundary_node)
                .cloned()
                .collect_vec();

            Ok(available_nodes
                .iter()
                .filter(|n| {
                    // Keep only healthy nodes.
                    healths
                        .get(&n.principal)
                        .map(|s| matches!(*s, ic_management_types::HealthStatus::Healthy))
                        .unwrap_or(false)
                })
                .cloned()
                .sorted_by(|n1, n2| n1.principal.cmp(&n2.principal))
                .collect())
        })
    }
}

mock! {
    pub LazyRegistry {}
    impl LazyRegistry for LazyRegistry {
        fn node_labels(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<Guest>>>>;

        fn elected_guestos(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<String>>>>;

        fn elected_hostos(&self) -> BoxFuture<'_, anyhow::Result<Arc<Vec<String>>>>;

        fn sync_with_nns(&self) -> BoxFuture<'_, anyhow::Result<()>>;

        fn operators(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Operator>>>>;

        fn nodes(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Node>>>>;

        fn firewall_rule_set(&self, firewall_rule_scope: FirewallRulesScope) -> BoxFuture<'_, anyhow::Result<FirewallRuleSet>>;

        fn subnets(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Subnet>>>>;

        fn nodes_and_proposals(&self) -> BoxFuture<'_, anyhow::Result<Arc<IndexMap<PrincipalId, Node>>>>;

        fn unassigned_nodes_replica_version(&self) -> BoxFuture<'_, anyhow::Result<Arc<String>>>;

        fn get_api_boundary_nodes(&self) -> anyhow::Result<Vec<(String, ApiBoundaryNodeRecord)>>;

        fn get_node_rewards_table(&self) -> anyhow::Result<IndexMap<String, NodeRewardsTable>>;

        fn get_unassigned_nodes(&self) -> anyhow::Result<Option<UnassignedNodesConfigRecord>>;

        fn get_datacenters(&self) -> anyhow::Result<Vec<DataCenterRecord>>;

        fn elected_guestos_records(&self) -> anyhow::Result<Vec<ReplicaVersionRecord>>;

        fn elected_hostos_records(&self) -> anyhow::Result<Vec<HostosVersionRecord>>;

        fn update_proposal_data(&self) -> BoxFuture<'_, anyhow::Result<()>>;
    }

    impl LazyRegistryFamilyEntries for LazyRegistry {
        fn get_key_family(&self, key_prefix: &str, version: RegistryVersion) -> anyhow::Result<Vec<String>>;

        fn get_versioned_value(&self, key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>>;

        fn get_latest_version(&self) -> RegistryVersion;
    }

    impl NodesConverter for LazyRegistry {
        fn get_nodes<'a>(&'a self, from: &'a [PrincipalId]) -> BoxFuture<'_, Result<Vec<Node>, NetworkError>>;
    }

    impl SubnetQuerier for LazyRegistry {
        fn subnet(&self, by: SubnetQueryBy) -> BoxFuture<'_, Result<DecentralizedSubnet, NetworkError>>;
    }

    impl decentralization::network::TopologyManager for LazyRegistry {}

    impl AvailableNodesQuerier for LazyRegistry {
        fn available_nodes(&self) -> BoxFuture<'_, Result<Vec<Node>, NetworkError>>;
    }
}
