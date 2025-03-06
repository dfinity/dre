use crate::registry_store::{CanisterRegistryClient, CanisterRegistryStore};
use ic_base_types::{PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::subnet::v1::SubnetRecord;
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX};
use indexmap::IndexMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock, RwLockReadGuard};

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

pub struct RegistryClient<Memory: ic_stable_structures::Memory> {
    registry_store: Arc<RwLock<CanisterRegistryStore<Memory>>>,
}
impl<Memory> RegistryClient<Memory>
where
    Memory: ic_stable_structures::Memory,
{
    pub fn init(registry_store: Arc<RwLock<CanisterRegistryStore<Memory>>>) -> Self {
        Self { registry_store }
    }

    fn registry_store(&self) -> RwLockReadGuard<CanisterRegistryStore<Memory>> {
        self.registry_store.read().expect("Failed to lock registry store")
    }
    fn get_family_entries_of_version<T: RegistryEntry + Default>(&self, version: RegistryVersion) -> IndexMap<String, (u64, T)> {
        let prefix_length = T::KEY_PREFIX.len();
        self.registry_store()
            .get_key_family(T::KEY_PREFIX, version)
            .expect("Failed to get key family")
            .iter()
            .filter_map(|key| {
                let r = self
                    .registry_store()
                    .get_versioned_value(key, version)
                    .unwrap_or_else(|_| panic!("Failed to get entry {} for type {}", key, std::any::type_name::<T>()));

                r.as_ref().map(|v| {
                    (
                        key[prefix_length..].to_string(),
                        (r.version.get(), T::decode(v.as_slice()).expect("Invalid registry value")),
                    )
                })
            })
            .collect()
    }

    fn get_family_entries<T: RegistryEntry + Default>(&self) -> IndexMap<String, (u64, T)> {
        self.get_family_entries_of_version::<T>(self.registry_store().get_latest_version())
    }

    pub fn subnets_list(&self) -> Vec<SubnetId> {
        self.get_family_entries::<SubnetRecord>()
            .iter()
            .map(|(subnet_id, _)| PrincipalId::from_str(subnet_id).map(SubnetId::from).expect("Invalid subnet id"))
            .collect()
    }
}
