use std::str::FromStr;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use ic_interfaces_registry::RegistryClient;
use ic_interfaces_registry::{RegistryClientVersionedResult, RegistryValue};
use ic_management_types::{Datacenter, DatacenterOwner, Guest, Network, Node, NodeProvidersResponse, Operator, Provider, Subnet};
use ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions;
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord, dc::v1::DataCenterRecord, hostos_version::v1::HostosVersionRecord,
    replica_version::v1::ReplicaVersionRecord, subnet::v1::SubnetRecord, unassigned_nodes_config::v1::UnassignedNodesConfigRecord,
};
use ic_registry_client_helpers::{node::NodeRecord, node_operator::NodeOperatorRecord};
use ic_registry_keys::{
    API_BOUNDARY_NODE_RECORD_KEY_PREFIX, DATA_CENTER_KEY_PREFIX, HOSTOS_VERSION_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX,
    REPLICA_VERSION_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_local_registry::LocalRegistry;
use ic_types::{PrincipalId, RegistryVersion};
use itertools::Itertools;
use log::warn;

use crate::node_labels;
use crate::public_dashboard::query_ic_dashboard_list;
use crate::registry::RegistryFamilyEntries;

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
}
