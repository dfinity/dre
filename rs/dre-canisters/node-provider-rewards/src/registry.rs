use ic_base_types::{PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::subnet::v1::SubnetRecord;
use ic_registry_canister_client::{CanisterRegistryClient, RegistryDataStableMemory, StableCanisterRegistryClient};
use ic_registry_keys::{DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX};
use indexmap::IndexMap;
use node_provider_rewards::reward_period::TimestampNanos;
use node_provider_rewards::types::RewardableNode;
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

pub struct RegistryClient<S: RegistryDataStableMemory> {
    pub(crate) store: StableCanisterRegistryClient<S>,
}

impl<S: RegistryDataStableMemory> RegistryClient<S> {
    pub async fn schedule_registry_sync(&self) -> Result<RegistryVersion, String> {
        self.store.sync_registry_stored().await
    }

    fn get_family_entries_of_version<T: RegistryEntry + Default>(&self, version: RegistryVersion) -> IndexMap<String, (u64, T)> {
        let prefix_length = T::KEY_PREFIX.len();

        self.store
            .get_key_family(T::KEY_PREFIX, version)
            .expect("Failed to get key family")
            .iter()
            .filter_map(|key| {
                let r = self
                    .store
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
        self.get_family_entries_of_version::<T>(self.store.get_latest_version())
    }

    pub fn subnets_list(&self) -> Vec<SubnetId> {
        self.get_family_entries::<SubnetRecord>()
            .iter()
            .map(|(subnet_id, _)| PrincipalId::from_str(subnet_id).map(SubnetId::from).expect("Invalid subnet id"))
            .collect()
    }
}

pub fn get_rewards_table() -> NodeRewardsTable {
    let latest_version = RegistryStoreInstance::local_latest_version();

    get_versioned_value::<NodeRewardsTable>(NODE_REWARDS_TABLE_KEY, latest_version).expect("Failed to get subnets list")
}

fn estimate_node_type(rewardable_count: Option<&mut BTreeMap<String, u32>>) -> String {
    match rewardable_count {
        Some(rewardable_count) => {
            if rewardable_count.is_empty() {
                "unknown:no_rewardable_nodes_found".to_string()
            } else {
                let (k, mut v) = loop {
                    let (k, v) = match rewardable_count.pop_first() {
                        Some(kv) => kv,
                        None => break ("unknown:rewardable_nodes_used_up".to_string(), 0),
                    };
                    if v != 0 {
                        break (k, v);
                    }
                };
                v = v.saturating_sub(1);
                if v != 0 {
                    rewardable_count.insert(k.clone(), v);
                }
                k
            }
        }
        None => "unknown".to_string(),
    }
}

pub fn get_rewardable_nodes(_start_ts: TimestampNanos, _end_ts: TimestampNanos) -> Vec<RewardableNode> {
    let mut nodes = BTreeMap::new();
    let mut node_operator_rewardable_count = BTreeMap::new();

    // TODO: Extend to all the versions in the range once the registry supports it.
    // https://github.com/dfinity/ic/pull/4450
    let nodes_record = get_family_entries::<NodeRecord>();

    for (principal_id, (_, node_record)) in nodes_record {
        let node_id: NodeId = PrincipalId::from_str(principal_id.as_str()).unwrap().into();

        if nodes.get(&node_id).is_none() {
            let node_operator_id: PrincipalId = node_record.node_operator_id.try_into().unwrap();
            let node_operator_record = get_value::<NodeOperatorRecord>(&make_node_operator_record_key(node_operator_id).as_str()).unwrap();

            if node_operator_rewardable_count.get(&node_operator_id).is_none() {
                node_operator_rewardable_count.insert(node_operator_id, node_operator_record.rewardable_nodes);
            }

            let node_type = estimate_node_type(node_operator_rewardable_count.get_mut(&node_operator_id));

            let node_provider_id: PrincipalId = node_operator_record.node_provider_principal_id.try_into().unwrap();
            let data_center_record = get_value::<DataCenterRecord>(&make_data_center_record_key(&node_operator_record.dc_id)).unwrap();

            nodes.insert(
                node_id,
                RewardableNode {
                    node_id,
                    node_provider_id,
                    region: data_center_record.region,
                    node_type,
                },
            );
        }
    }
    nodes.into_values().collect()
}
