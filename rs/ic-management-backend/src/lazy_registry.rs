use std::collections::{BTreeSet, HashSet};
use std::net::Ipv6Addr;
use std::str::FromStr;
use std::sync::Arc;
use std::{cell::RefCell, collections::BTreeMap};

use decentralization::network::{AvailableNodesQuerier, DecentralizedSubnet, NodesConverter, SubnetQuerier, SubnetQueryBy};
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
use log::warn;
use tokio::try_join;

use crate::health::HealthStatusQuerier;
use crate::public_dashboard::query_ic_dashboard_list;
use crate::registry::{DFINITY_DCS, NNS_SUBNET_NAME};
use crate::{node_labels, proposal};

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

pub struct LazyRegistry {
    local_registry: LocalRegistry,
    network: Network,

    subnets: RefCell<Option<Arc<BTreeMap<PrincipalId, Subnet>>>>,
    nodes: RefCell<Option<Arc<BTreeMap<PrincipalId, Node>>>>,
    operators: RefCell<Option<Arc<BTreeMap<PrincipalId, Operator>>>>,
    node_labels_guests: RefCell<Option<Arc<Vec<Guest>>>>,
    elected_guestos: RefCell<Option<Arc<Vec<String>>>>,
    elected_hostos: RefCell<Option<Arc<Vec<String>>>>,
    unassigned_nodes_replica_version: RefCell<Option<Arc<String>>>,
    firewall_rule_set: RefCell<Option<Arc<BTreeMap<String, FirewallRuleSet>>>>,
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

    fn get_family_entries<T: LazyRegistryEntry + Default>(&self) -> anyhow::Result<BTreeMap<String, T>> {
        let family = self.get_family_entries_versioned::<T>()?;
        Ok(family.into_iter().map(|(k, (_, v))| (k, v)).collect())
    }
    fn get_family_entries_versioned<T: LazyRegistryEntry + Default>(&self) -> anyhow::Result<BTreeMap<String, (u64, T)>> {
        self.get_family_entries_of_version(self.get_latest_version())
    }
    fn get_family_entries_of_version<T: LazyRegistryEntry + Default>(&self, version: RegistryVersion) -> anyhow::Result<BTreeMap<String, (u64, T)>> {
        let prefix_length = T::KEY_PREFIX.len();
        Ok(self
            .get_key_family(T::KEY_PREFIX, version)?
            .iter()
            .filter_map(|key| {
                let r = self
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
}

impl LazyRegistryFamilyEntries for LazyRegistry {
    fn get_key_family(&self, key_prefix: &str, version: RegistryVersion) -> anyhow::Result<Vec<String>> {
        Ok(self.local_registry.get_key_family(key_prefix, version)?)
    }

    fn get_versioned_value(&self, key: &str, version: RegistryVersion) -> RegistryClientVersionedResult<Vec<u8>> {
        self.local_registry.get_versioned_value(key, version)
    }

    fn get_latest_version(&self) -> RegistryVersion {
        self.local_registry.get_latest_version()
    }
}

impl LazyRegistry {
    pub fn new(local_registry: LocalRegistry, network: Network) -> Self {
        Self {
            local_registry,
            network,
            subnets: RefCell::new(None),
            nodes: RefCell::new(None),
            operators: RefCell::new(None),
            node_labels_guests: RefCell::new(None),
            elected_guestos: RefCell::new(None),
            elected_hostos: RefCell::new(None),
            unassigned_nodes_replica_version: RefCell::new(None),
            firewall_rule_set: RefCell::new(None),
        }
    }

    // See if making it async would change anything
    pub async fn node_labels(&self) -> anyhow::Result<Arc<Vec<Guest>>> {
        if let Some(guests) = self.node_labels_guests.borrow().as_ref() {
            return Ok(guests.to_owned());
        }

        let guests = match node_labels::query_guests(&self.network.name).await {
            Ok(g) => g,
            Err(e) => {
                warn!("Failed to query node labels: {}", e);
                vec![]
            }
        };

        let guests = Arc::new(guests);
        *self.node_labels_guests.borrow_mut() = Some(guests.clone());
        Ok(guests)
    }

    pub fn elected_guestos(&self) -> anyhow::Result<Arc<Vec<String>>> {
        if let Some(elected) = self.elected_guestos.borrow().as_ref() {
            return Ok(elected.to_owned());
        }

        let record = self
            .get_family_entries::<BlessedReplicaVersions>()?
            .first_entry()
            .ok_or(anyhow::anyhow!("No blessed replica versions found"))?
            .get()
            .to_owned();

        let record = Arc::new(record.blessed_version_ids);
        *self.elected_guestos.borrow_mut() = Some(record.clone());
        Ok(record)
    }

    pub fn elected_hostos(&self) -> anyhow::Result<Arc<Vec<String>>> {
        if let Some(elected) = self.elected_hostos.borrow().as_ref() {
            return Ok(elected.to_owned());
        }

        let record = self
            .get_family_entries::<HostosVersionRecord>()?
            .values()
            .map(|v| v.hostos_version_id.to_owned())
            .collect_vec();

        let record = Arc::new(record);
        *self.elected_hostos.borrow_mut() = Some(record.clone());
        Ok(record)
    }

    pub async fn operators(&self) -> anyhow::Result<Arc<BTreeMap<PrincipalId, Operator>>> {
        if let Some(operators) = self.operators.borrow().as_ref() {
            return Ok(operators.to_owned());
        }

        // Fetch node providers

        let node_providers = query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers").await?;
        let node_providers: BTreeMap<_, _> = node_providers.node_providers.iter().map(|p| (p.principal_id, p)).collect();
        let data_centers = self.get_family_entries::<DataCenterRecord>()?;
        let operators = self.get_family_entries::<NodeOperatorRecord>()?;

        let records: BTreeMap<_, _> = operators
            .iter()
            .map(|(p, or)| {
                let principal = PrincipalId::from_str(p).expect("Invalid operator principal id");
                (
                    principal,
                    Operator {
                        principal,
                        provider: PrincipalId::try_from(or.node_provider_principal_id.as_slice())
                            .map(|p| {
                                let maybe_provider = node_providers.get(&p).map(|node_provider| Provider {
                                    name: Some(node_provider.display_name.to_owned()),
                                    website: node_provider.website.to_owned(),
                                    principal: p,
                                });

                                if maybe_provider.is_none() && self.network.is_mainnet() {
                                    panic!("Node provider not found for operator: {}", principal);
                                }
                                maybe_provider.unwrap_or_default()
                            })
                            .unwrap(),
                        allowance: or.node_allowance,
                        datacenter: data_centers.get(&or.dc_id).map(|dc| {
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

        let records = Arc::new(records);
        *self.operators.borrow_mut() = Some(records.clone());
        Ok(records)
    }

    pub async fn nodes(&self) -> anyhow::Result<Arc<BTreeMap<PrincipalId, Node>>> {
        if let Some(nodes) = self.nodes.borrow().as_ref() {
            return Ok(nodes.to_owned());
        }

        let node_entries = self.get_family_entries::<NodeRecord>()?;
        let versioned_node_entries = self.get_family_entries_versioned::<NodeRecord>()?;
        let dfinity_dcs = DFINITY_DCS.split(' ').map(|dc| dc.to_string().to_lowercase()).collect::<HashSet<_>>();
        let api_boundary_nodes = self.get_family_entries::<ApiBoundaryNodeRecord>()?;
        let guests = self.node_labels().await?;
        let operators = self.operators().await?;
        let nodes: BTreeMap<_, _> = node_entries
            .iter()
            // Skipping nodes without operator. This should only occur at version 1
            .filter(|(_, nr)| !nr.node_operator_id.is_empty())
            .map(|(p, nr)| {
                let guest = Self::node_record_guest(guests.clone(), nr);
                let operator = operators
                    .iter()
                    .find(|(op, _)| op.to_vec() == nr.node_operator_id)
                    .map(|(_, o)| o.to_owned())
                    .expect("Missing operator referenced by a node");

                let principal = PrincipalId::from_str(p).expect("Invalid node principal id");
                let ip_addr = Self::node_ip_addr(nr);
                let dc_name = match &operator.datacenter {
                    Some(dc) => dc.name.to_lowercase(),
                    None => "".to_string(),
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
                        // TODO: map hostos release
                        hostos_release: None,
                        operator,
                        proposal: None,
                        label: guest.map(|g| g.name),
                        decentralized: ip_addr.segments()[4] == 0x6801,
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
                    },
                )
            })
            .collect();

        let nodes = Arc::new(nodes);
        *self.nodes.borrow_mut() = Some(nodes.clone());
        Ok(nodes)
    }

    pub fn firewall_rule_set(&self, firewall_rule_scope: FirewallRulesScope) -> anyhow::Result<FirewallRuleSet> {
        let key = make_firewall_rules_record_key(&firewall_rule_scope);
        if let Some(firewall_rule_set) = self.firewall_rule_set.borrow().as_ref() {
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

        let mut opt_arc_map = self.firewall_rule_set.borrow_mut();
        if let Some(arc_map) = opt_arc_map.as_mut() {
            let bag = Arc::make_mut(arc_map);
            bag.insert(key.to_owned(), value.clone());
        } else {
            let mut all = BTreeMap::new();
            all.insert(key.to_owned(), value.clone());
            *opt_arc_map = Some(Arc::new(all));
        }

        Ok(value)
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

    pub async fn subnets(&self) -> anyhow::Result<Arc<BTreeMap<PrincipalId, Subnet>>> {
        if let Some(subnets) = self.subnets.borrow().as_ref() {
            return Ok(subnets.to_owned());
        }

        let all_nodes = self.nodes().await?;

        let subnets: BTreeMap<_, _> = self
            .get_family_entries::<SubnetRecord>()?
            .iter()
            .enumerate()
            .map(|(i, (p, sr))| {
                let principal = PrincipalId::from_str(p).expect("Invalid subnet principal id");
                let subnet_nodes = all_nodes
                    .iter()
                    .filter(|(_, n)| n.subnet_id.map_or(false, |s| s == principal))
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
                    },
                )
            })
            .filter(|(_, s)| !s.nodes.is_empty())
            .collect();

        let subnets = Arc::new(subnets);
        *self.subnets.borrow_mut() = Some(subnets.clone());
        Ok(subnets)
    }

    pub async fn nodes_with_proposals(&self) -> anyhow::Result<Arc<BTreeMap<PrincipalId, Node>>> {
        let nodes = self.nodes().await?;
        if nodes.iter().any(|(_, n)| n.proposal.is_some()) {
            return Ok(nodes);
        }

        self.update_proposal_data().await?;
        self.nodes().await
    }

    pub async fn subnets_with_proposals(&self) -> anyhow::Result<Arc<BTreeMap<PrincipalId, Subnet>>> {
        let subnets = self.subnets().await?;

        if subnets.iter().any(|(_, s)| s.proposal.is_some()) {
            return Ok(subnets);
        }

        self.update_proposal_data().await?;
        self.subnets().await
    }

    async fn update_proposal_data(&self) -> anyhow::Result<()> {
        let proposal_agent = proposal::ProposalAgent::new(self.network.get_nns_urls());
        let nodes = self.nodes().await?;
        let subnets = self.subnets().await?;

        let topology_proposals = proposal_agent.list_open_topology_proposals().await?;
        let nodes: BTreeMap<_, _> = nodes
            .iter()
            .map(|(p, n)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|p| p.node_ids_added.contains(&n.principal) || p.node_ids_removed.contains(&n.principal))
                    .cloned();

                (*p, Node { proposal, ..n.clone() })
            })
            .collect();

        let subnets: BTreeMap<_, _> = subnets
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

        *self.nodes.borrow_mut() = Some(Arc::new(nodes));
        *self.subnets.borrow_mut() = Some(Arc::new(subnets));

        Ok(())
    }

    pub async fn nns_replica_version(&self) -> anyhow::Result<Option<String>> {
        Ok(self
            .subnets()
            .await?
            .get(&PrincipalId::from_str("tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe").unwrap())
            .map(|s| s.replica_version.clone()))
    }

    pub async fn missing_guests(&self) -> anyhow::Result<Vec<Guest>> {
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
    }

    pub async fn get_decentralized_nodes(&self, principals: &[PrincipalId]) -> anyhow::Result<Vec<decentralization::network::Node>> {
        Ok(self
            .nodes()
            .await?
            .values()
            .filter(|n| principals.contains(&n.principal))
            .map(decentralization::network::Node::from)
            .collect_vec())
    }

    pub fn unassigned_nodes_replica_version(&self) -> anyhow::Result<Arc<String>> {
        if let Some(v) = self.unassigned_nodes_replica_version.borrow().as_ref() {
            return Ok(v.to_owned());
        }

        let version = self
            .get_family_entries::<UnassignedNodesConfigRecord>()?
            .first_entry()
            .map(|v| v.get().to_owned())
            .ok_or(anyhow::anyhow!("No unassigned nodes version"))?;

        let version = Arc::new(version.replica_version);
        *self.unassigned_nodes_replica_version.borrow_mut() = Some(version.clone());
        Ok(version)
    }
}

impl NodesConverter for LazyRegistry {
    async fn get_nodes(&self, from: &[PrincipalId]) -> Result<Vec<decentralization::network::Node>, ic_management_types::NetworkError> {
        let nodes = self
            .nodes()
            .await
            .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?;
        from.iter()
            .map(|n| {
                nodes
                    .get(n)
                    .ok_or(ic_management_types::NetworkError::NodeNotFound(*n))
                    .map(decentralization::network::Node::from)
            })
            .collect()
    }
}

impl SubnetQuerier for LazyRegistry {
    async fn subnet(&self, by: SubnetQueryBy) -> Result<DecentralizedSubnet, ic_management_types::NetworkError> {
        match by {
            SubnetQueryBy::SubnetId(id) => self
                .subnets()
                .await
                .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?
                .get(&id)
                .map(|s| DecentralizedSubnet {
                    id: s.principal,
                    nodes: s.nodes.iter().map(decentralization::network::Node::from).collect(),
                    removed_nodes: vec![],
                    min_nakamoto_coefficients: None,
                    comment: None,
                    run_log: vec![],
                })
                .ok_or(ic_management_types::NetworkError::SubnetNotFound(id)),
            SubnetQueryBy::NodeList(nodes) => {
                let reg_nodes = self.nodes().await.map_err(|e| NetworkError::DataRequestError(e.to_string()))?;
                let subnets = nodes
                    .iter()
                    .map(|n| reg_nodes.get(&n.id).and_then(|n| n.subnet_id))
                    .collect::<BTreeSet<_>>();
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
                            .iter()
                            .map(decentralization::network::Node::from)
                            .collect(),
                        removed_nodes: vec![],
                        min_nakamoto_coefficients: None,
                        comment: None,
                        run_log: vec![],
                    })
                } else {
                    Err(NetworkError::IllegalRequest("no subnets found".to_string()))
                }
            }
        }
    }
}
impl decentralization::network::TopologyManager for LazyRegistry {}

impl AvailableNodesQuerier for LazyRegistry {
    async fn available_nodes(&self) -> Result<Vec<decentralization::network::Node>, ic_management_types::NetworkError> {
        let health_client = crate::health::HealthClient::new(self.network.clone());
        let (nodes, healths) = try_join!(self.nodes_with_proposals(), health_client.nodes())
            .map_err(|e| ic_management_types::NetworkError::DataRequestError(e.to_string()))?;
        let nodes = nodes
            .values()
            .filter(|n| n.subnet_id.is_none() && n.proposal.is_none() && n.duplicates.is_none() && !n.is_api_boundary_node)
            .cloned()
            .collect_vec();

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
