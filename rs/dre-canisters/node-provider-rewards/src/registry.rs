use crate::RegistryStoreInstance;
use ic_base_types::{PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::subnet::v1::SubnetRecord;
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX};
use indexmap::IndexMap;
use std::str::FromStr;

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

fn get_family_entries_of_version<T: RegistryEntry + Default>(version: RegistryVersion) -> IndexMap<String, (u64, T)> {
    let prefix_length = T::KEY_PREFIX.len();

    RegistryStoreInstance::get_key_family(T::KEY_PREFIX, version)
        .expect("Failed to get key family")
        .iter()
        .filter_map(|key| {
            let r = RegistryStoreInstance::get_versioned_value(key, version)
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

fn get_family_entries<T: RegistryEntry + Default>() -> IndexMap<String, (u64, T)> {
    let latest_version = RegistryStoreInstance::local_latest_version();
    get_family_entries_of_version::<T>(latest_version)
}

pub fn subnets_list() -> Vec<SubnetId> {
    get_family_entries::<SubnetRecord>()
        .iter()
        .map(|(subnet_id, _)| PrincipalId::from_str(subnet_id).map(SubnetId::from).expect("Invalid subnet id"))
        .collect()
}

pub fn get_rewards_table() -> Vec<SubnetId> {
    get_family_entries::<SubnetRecord>()
        .iter()
        .map(|(subnet_id, _)| PrincipalId::from_str(subnet_id).map(SubnetId::from).expect("Invalid subnet id"))
        .collect()
}
