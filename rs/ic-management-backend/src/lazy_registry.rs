use std::collections::HashSet;
use std::net::Ipv6Addr;
use std::str::FromStr;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use ic_interfaces_registry::RegistryClient;
use ic_interfaces_registry::{RegistryClientVersionedResult, RegistryValue};
use ic_management_types::{Datacenter, DatacenterOwner, Guest, Network, Node, NodeProvidersResponse, Operator, Provider, Subnet, SubnetMetadata};
use ic_nns_constants::SUBNET_RENTAL_CANISTER_ID;
use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_protobuf::registry::subnet;
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord, dc::v1::DataCenterRecord, hostos_version::v1::HostosVersionRecord,
    replica_version::v1::ReplicaVersionRecord, subnet::v1::SubnetRecord, unassigned_nodes_config::v1::UnassignedNodesConfigRecord,
};
use ic_registry_client_helpers::node::NodeRegistry;
use ic_registry_client_helpers::subnet::SubnetListRegistry;
use ic_registry_client_helpers::{node::NodeRecord, node_operator::NodeOperatorRecord};
use ic_registry_keys::{
    API_BOUNDARY_NODE_RECORD_KEY_PREFIX, DATA_CENTER_KEY_PREFIX, HOSTOS_VERSION_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX,
    REPLICA_VERSION_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_local_registry::LocalRegistry;
use ic_registry_subnet_type::SubnetType;
use ic_types::{NodeId, PrincipalId, RegistryVersion};
use itertools::Itertools;
use log::warn;

use crate::public_dashboard::query_ic_dashboard_list;
use crate::registry::{RegistryFamilyEntries, DFINITY_DCS, NNS_SUBNET_NAME};
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

    subnets: RefCell<Option<Rc<BTreeMap<PrincipalId, Subnet>>>>,
    nodes: RefCell<Option<Rc<BTreeMap<PrincipalId, Node>>>>,
    operators: RefCell<Option<Rc<BTreeMap<PrincipalId, Operator>>>>,
    node_labels_guests: RefCell<Option<Rc<Vec<Guest>>>>,
    known_subnets: RefCell<Option<Rc<BTreeMap<PrincipalId, String>>>>,
    elected_guestos: RefCell<Option<Rc<Vec<String>>>>,
    elected_hostos: RefCell<Option<Rc<Vec<String>>>>,
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
            known_subnets: RefCell::new(None),
            elected_guestos: RefCell::new(None),
            elected_hostos: RefCell::new(None),
        }
    }

    // See if making it async would change anything
    pub fn node_labels(&self) -> anyhow::Result<Rc<Vec<Guest>>> {
        if let Some(guests) = self.node_labels_guests.borrow().as_ref() {
            return Ok(guests.to_owned());
        }

        let guests = match tokio::runtime::Handle::current().block_on(node_labels::query_guests(&self.network.name)) {
            Ok(g) => g,
            Err(e) => {
                warn!("Failed to query node labels: {}", e);
                vec![]
            }
        };

        let guests = Rc::new(guests);
        *self.node_labels_guests.borrow_mut() = Some(guests.clone());
        Ok(guests)
    }

    pub fn elected_guestos(&self) -> anyhow::Result<Rc<Vec<String>>> {
        if let Some(elected) = self.elected_guestos.borrow().as_ref() {
            return Ok(elected.to_owned());
        }

        let record = self
            .get_family_entries::<BlessedReplicaVersions>()?
            .first_entry()
            .ok_or(anyhow::anyhow!("No blessed replica versions found"))?
            .get()
            .to_owned();

        let record = Rc::new(record.blessed_version_ids);
        *self.elected_guestos.borrow_mut() = Some(record.clone());
        Ok(record)
    }

    pub fn elected_hostos(&self) -> anyhow::Result<Rc<Vec<String>>> {
        if let Some(elected) = self.elected_hostos.borrow().as_ref() {
            return Ok(elected.to_owned());
        }

        let record = self
            .get_family_entries::<HostosVersionRecord>()?
            .iter()
            .map(|(_, v)| v.hostos_version_id.to_owned())
            .collect_vec();

        let record = Rc::new(record);
        *self.elected_hostos.borrow_mut() = Some(record.clone());
        Ok(record)
    }

    pub fn operators(&self) -> anyhow::Result<Rc<BTreeMap<PrincipalId, Operator>>> {
        if let Some(operators) = self.operators.borrow().as_ref() {
            return Ok(operators.to_owned());
        }

        // Fetch node providers

        let node_providers =
            tokio::runtime::Handle::current().block_on(query_ic_dashboard_list::<NodeProvidersResponse>(&self.network, "v3/node-providers"))?;
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
                                node_providers
                                    .get(&p)
                                    .map(|node_provider| Provider {
                                        name: Some(node_provider.display_name.to_owned()),
                                        website: node_provider.website.to_owned(),
                                        principal: p,
                                    })
                                    .expect("Provider missing for operator record")
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

        let records = Rc::new(records);
        *self.operators.borrow_mut() = Some(records.clone());
        Ok(records)
    }

    pub fn nodes(&self) -> anyhow::Result<Rc<BTreeMap<PrincipalId, Node>>> {
        if let Some(nodes) = self.nodes.borrow().as_ref() {
            return Ok(nodes.to_owned());
        }

        let node_entries = self.get_family_entries::<NodeRecord>()?;
        let versioned_node_entries = self.get_family_entries_versioned::<NodeRecord>()?;
        let dfinity_dcs = DFINITY_DCS.split(' ').map(|dc| dc.to_string().to_lowercase()).collect::<HashSet<_>>();
        let api_boundary_nodes = self.get_family_entries::<ApiBoundaryNodeRecord>()?;

        let nodes: BTreeMap<_, _> = node_entries
            .iter()
            // Skipping nodes without operator. This should only occur at version 1
            .filter(|(_, nr)| !nr.node_operator_id.is_empty())
            .map(|(p, nr)| {
                let guest = self.node_record_guest(nr);
                let operator = self
                    .operators()
                    .expect("Should be able to fetch operators")
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

        let nodes = Rc::new(nodes);
        *self.nodes.borrow_mut() = Some(nodes.clone());
        Ok(nodes)
    }

    fn node_record_guest(&self, nr: &NodeRecord) -> Option<Guest> {
        match self.node_labels() {
            Ok(guests) => guests,
            Err(_) => Rc::new(vec![]),
        }
        .iter()
        .find(|g| g.ipv6 == Ipv6Addr::from_str(&nr.http.clone().unwrap().ip_addr).unwrap())
        .cloned()
    }

    fn node_ip_addr(nr: &NodeRecord) -> Ipv6Addr {
        Ipv6Addr::from_str(&nr.http.clone().expect("missing ipv6 address").ip_addr).expect("invalid ipv6 address")
    }

    pub fn subnets(&self) -> anyhow::Result<Rc<BTreeMap<PrincipalId, Subnet>>> {
        if let Some(subnets) = self.subnets.borrow().as_ref() {
            return Ok(subnets.to_owned());
        }

        let all_nodes = self.nodes()?;

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

        let subnets = Rc::new(subnets);
        *self.subnets.borrow_mut() = Some(subnets.clone());
        Ok(subnets)
    }

    pub fn nodes_with_proposals(&self) -> anyhow::Result<Rc<BTreeMap<PrincipalId, Node>>> {
        let nodes = self.nodes()?;
        if nodes.iter().any(|(_, n)| n.proposal.is_some()) {
            return Ok(nodes);
        }

        let proposal_agent = proposal::ProposalAgent::new(self.network.get_nns_urls());

        let topology_proposals = tokio::runtime::Handle::current().block_on(proposal_agent.list_open_topology_proposals())?;
        let nodes: BTreeMap<_, _> = nodes
            .iter()
            .map(|(p, n)| {
                let proposal = topology_proposals
                    .iter()
                    .find(|p| p.node_ids_added.contains(&n.principal) || p.node_ids_removed.contains(&n.principal))
                    .cloned();

                (p.clone(), Node { proposal, ..n.clone() })
            })
            .collect();

        let nodes = Rc::new(nodes);
        *self.nodes.borrow_mut() = Some(nodes.clone());
        Ok(nodes)
    }
}
