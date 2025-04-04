use ic_base_types::{NodeId, PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::subnet::v1::{SubnetListRecord, SubnetRecord};
use ic_registry_canister_client::{CanisterRegistryClient, RegistryDataStableMemory, StableCanisterRegistryClient};
use ic_registry_keys::{
    make_data_center_record_key, make_node_operator_record_key, make_subnet_list_record_key, DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX,
    NODE_RECORD_KEY_PREFIX, NODE_REWARDS_TABLE_KEY, SUBNET_RECORD_KEY_PREFIX,
};
use ic_types::registry::RegistryClientError;
use indexmap::IndexMap;
use rewards_calculation::types::RewardableNode;
use rewards_calculation::types::RewardableNode;
use rewards_calculation::types::TimestampNanos;
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

    pub fn get_versioned_value<T: RegistryValue + Default>(&self, key: &str, version: RegistryVersion) -> Result<T, RegistryClientError> {
        let value = self
            .store
            .get_versioned_value(key, version)?
            .map(|v| T::decode(v.as_slice()).unwrap())
            .value
            .unwrap_or_default();
        Ok(value)
    }

    pub fn get_value<T: RegistryValue + Default>(&self, key: &str) -> Result<T, RegistryClientError> {
        self.get_versioned_value::<T>(key, self.store.get_latest_version())
    }

    pub fn subnets_list(&self) -> Vec<SubnetId> {
        let record = self
            .get_value::<SubnetListRecord>(make_subnet_list_record_key().as_str())
            .expect("Failed to get subnets list");

        record
            .subnets
            .into_iter()
            .map(|s| SubnetId::from(PrincipalId::try_from(s.clone().as_slice()).unwrap()))
            .collect()
    }

    pub fn get_rewards_table(&self) -> NodeRewardsTable {
        self.get_value::<NodeRewardsTable>(NODE_REWARDS_TABLE_KEY)
            .expect("Failed to get NodeRewardsTable")
    }

    fn estimate_node_type(&self, rewardable_count: Option<&mut BTreeMap<String, u32>>) -> String {
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

    pub fn get_rewardable_nodes_per_provider(
        &self,
        _start_ts: TimestampNanos,
        _end_ts: TimestampNanos,
    ) -> BTreeMap<PrincipalId, Vec<RewardableNode>> {
        let mut rewardable_nodes_per_provider = BTreeMap::new();
        let mut node_operator_rewardable_count = BTreeMap::new();

        // TODO: Extend to all the versions in the range once the registry supports it.
        // https://github.com/dfinity/ic/pull/4450
        let nodes_record = self.get_family_entries::<NodeRecord>();

        for (principal_id, (_, node_record)) in nodes_record {
            let node_id: NodeId = PrincipalId::from_str(principal_id.as_str()).unwrap().into();
            let node_operator_id: PrincipalId = node_record.node_operator_id.try_into().unwrap();
            let node_operator_record = self
                .get_value::<NodeOperatorRecord>(make_node_operator_record_key(node_operator_id).as_str())
                .unwrap();

            if let Entry::Vacant(e) = node_operator_rewardable_count.entry(node_operator_id) {
                e.insert(node_operator_record.rewardable_nodes);
            }

            let node_type = self.estimate_node_type(node_operator_rewardable_count.get_mut(&node_operator_id));

            let node_provider_id: PrincipalId = node_operator_record.node_provider_principal_id.try_into().unwrap();
            let data_center_record = self
                .get_value::<DataCenterRecord>(&make_data_center_record_key(&node_operator_record.dc_id))
                .unwrap();

            rewardable_nodes_per_provider.entry(node_provider_id).or_default().push(RewardableNode {
                node_id,
                node_type: node_type.clone(),
                region: data_center_record.region,
            })
        }
        rewardable_nodes_per_provider
    }
}
