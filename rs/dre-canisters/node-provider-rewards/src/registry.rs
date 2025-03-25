use crate::RegistryStoreInstance;
use ic_base_types::{PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::subnet::v1::SubnetRecord;
use ic_registry_keys::{
    make_data_center_record_key, make_node_operator_record_key, DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX,
    NODE_REWARDS_TABLE_KEY, SUBNET_RECORD_KEY_PREFIX,
};
use indexmap::IndexMap;
use node_provider_rewards::reward_period::TimestampNanos;
use node_provider_rewards::types::RewardableNode;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
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

impl RegistryEntry for NodeRewardsTable {
    const KEY_PREFIX: &'static str = NODE_REWARDS_TABLE_KEY;
}

fn get_family_entries_between_versions<T: RegistryEntry + Default>(
    version_start: Option<RegistryVersion>,
    version_end: RegistryVersion,
) -> IndexMap<String, (u64, T)> {
    let prefix_length = T::KEY_PREFIX.len();

    if version_start.is_some() {
        RegistryStoreInstance::get_key_family_between_versions(T::KEY_PREFIX, version_start.unwrap(), version_end)
    } else {
        RegistryStoreInstance::get_key_family(T::KEY_PREFIX, version_end)
    }
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

fn get_family_entries_of_version<T: RegistryEntry + Default>(version: RegistryVersion) -> IndexMap<String, (u64, T)> {
    get_family_entries_between_versions::<T>(None, version)
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

pub fn get_rewards_table() -> NodeRewardsTable {
    get_family_entries::<NodeRewardsTable>()
        .into_values()
        .map(|(_, v)| v)
        .next()
        .unwrap_or_else(|| {
            panic!("Registry does not have a record for NodeRewardsTable.");
        })
}

pub fn get_rewardable_nodes(start_ts: TimestampNanos, end_ts: TimestampNanos) -> Vec<RewardableNode> {
    let mut nodes = BTreeMap::new();
    let mut rewardable_nodes = BTreeMap::new();

    // TODO: Extend to all the versions in the range once the registry supports it.
    // https://github.com/dfinity/ic/pull/4450
    let versions = vec![ReRegistryStoreInstance::local_latest_version()];

    for version in versions {
        let nodes_in_version = get_family_entries_of_version::<NodeRecord>(version);

        for (p, (_, node_record)) in nodes_in_version {
            let principal_id = PrincipalId::from_str(p.as_str()).unwrap();

            if let Entry::Vacant(node) = nodes.entry(principal_id) {
                let node_operator_id: PrincipalId = node_record.node_operator_id.try_into().unwrap();
                let key = make_node_operator_record_key(node_operator_id);
                let node_operator_record = get_family_entries_of_version::<NodeOperatorRecord>(version);

                if let Entry::Vacant(rewardables) = rewardable_nodes.entry(node_operator_id) {
                    rewardables.insert(node_operator_record.rewardable_nodes);
                }

                let node_provider_id: PrincipalId = node_operator_record.node_provider_principal_id.try_into().unwrap();
                let key = make_data_center_record_key(&node_operator_record.dc_id);
                let data_center_record = self
                    .local_registry
                    .get_versioned_value::<DataCenterRecord>(key.as_str(), registry_version)
                    .unwrap();

                node.insert(Node {
                    node_id: principal,
                    node_provider_id,
                    region: data_center_record.region,
                    node_type: match rewardable_nodes.get_mut(&node_operator_id) {
                        Some(rewardable_nodes) => {
                            if rewardable_nodes.is_empty() {
                                "unknown:no_rewardable_nodes_found".to_string()
                            } else {
                                let (k, mut v) = loop {
                                    let (k, v) = match rewardable_nodes.pop_first() {
                                        Some(kv) => kv,
                                        None => break ("unknown:rewardable_nodes_used_up".to_string(), 0),
                                    };
                                    if v != 0 {
                                        break (k, v);
                                    }
                                };
                                v = v.saturating_sub(1);
                                if v != 0 {
                                    rewardable_nodes.insert(k.clone(), v);
                                }
                                k
                            }
                        }

                        None => "unknown".to_string(),
                    },
                });
            }
        }
    }
    nodes.values().cloned().collect_vec()
}
