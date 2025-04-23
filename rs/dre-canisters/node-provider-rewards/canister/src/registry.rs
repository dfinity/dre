use ic_base_types::{NodeId, PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::subnet::v1::{SubnetListRecord, SubnetRecord};
use ic_registry_canister_client::{CanisterRegistryClient, RegistryDataStableMemory, StableCanisterRegistryClient, StorableRegistryKey, StorableRegistryValue};
use ic_registry_keys::{
    make_data_center_record_key, make_node_operator_record_key, make_subnet_list_record_key, DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX,
    NODE_RECORD_KEY_PREFIX, NODE_REWARDS_TABLE_KEY, SUBNET_RECORD_KEY_PREFIX,
};
use ic_types::registry::RegistryClientError;
use rewards_calculation::types::{RewardableNode, TimestampNanos};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use rewards_calculation::rewards_calculator_results::DayUTC;

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

struct RegistryVersionBounds {
    start_bound: RegistryVersion,
    end_bound: RegistryVersion,
}

impl<S: RegistryDataStableMemory> RegistryClient<S> {
    pub async fn schedule_registry_sync(&self) -> Result<RegistryVersion, String> {
        self.store.sync_registry_stored().await
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

    fn nodes_in_range(
        &self,
        registry_version_range: RegistryVersionBounds,
    ) -> Result<HashMap<NodeId, (NodeRecord, RegistryVersionBounds)>, RegistryClientError> {
        let version_end = registry_version_range.end_bound.get();
        let version_start = registry_version_range.start_bound.get();

        let key_prefix = NodeRecord::KEY_PREFIX;
        let prefix_length = key_prefix.len();
        let start_range = StorableRegistryKey::new(key_prefix.to_string(), Default::default());

        let mut registered_between_versions = HashMap::new();
        S::with_registry_map(|map| {
            for (node_key, StorableRegistryValue(maybe_value)) in map
                .range(start_range..)
                .filter(|(k, _)| k.version <= version_end)
                .take_while(|(k, _)| k.key.starts_with(key_prefix))
            {
                // Before rewarding period
                if node_key.version <= version_start {
                    if let Some(value) = maybe_value {
                        // Add nodes entry assuming registered for the entire rewards period
                        registered_between_versions.insert(node_key.key, (version_start, version_end, value));
                    } else {
                        // Remove nodes entry if it is not valid anymore
                        registered_between_versions.remove(&node_key.key);
                    }
                    continue;
                }

                // Inside rewards period
                if let Some(entry) = registered_between_versions.get_mut(&node_key.key) {
                    let (_, valid_to, present_value) = entry;
                    match maybe_value {
                        Some(value) => *present_value = value,
                        // If the node gets deleted the `node_key.version` represents the last version
                        // where the node was registered.
                        None => *valid_to = node_key.version,
                    }
                } else if let Some(value) = maybe_value {
                    // Handle case where the node is registered in the reward period
                    registered_between_versions.insert(node_key.key, (node_key.version, version_end, value));
                }
            }
        });

        Ok(
            registered_between_versions
                .into_iter()
                .map(|(node_id_key, (valid_from, valid_to, node_record))| {
                    let node_id = node_id_key[prefix_length..].to_string();
                    let node_id = NodeId::from(PrincipalId::from_str(&node_id).expect("Failed to parse node id"));
                    let version_bounds = RegistryVersionBounds {
                        start_bound: RegistryVersion::from(valid_from),
                        end_bound: RegistryVersion::from(valid_to),
                    };
                    let node_record = NodeRecord::decode(node_record.as_slice()).expect("Failed to decode node record");
                    (node_id, (node_record, version_bounds))
                })
                .collect()
        )
    }
    

    pub fn get_rewardable_nodes_per_provider(&self, start_ts: TimestampNanos, end_ts: TimestampNanos) -> Result<BTreeMap<PrincipalId, Vec<RewardableNode>>, RegistryClientError> {
        // TODO: Replace to cover all the versions in the reward period once the registry supports it.
        // https://github.com/dfinity/ic/pull/4450
        let mut rewardable_nodes_per_provider: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let end_bound = self.store.get_latest_version();
        let start_bound = RegistryVersion::from(end_bound.get() - 100);
        let nodes_in_range = self.nodes_in_range(
            RegistryVersionBounds {
                start_bound,
                end_bound,
            },
        )?;

        let mut node_operator_rewardable_count = BTreeMap::new();
        for (node_id, (node_record, versions_in_registry)) in nodes_in_range {
            let node_operator_id: PrincipalId = node_record.node_operator_id.try_into().unwrap();
            let node_operator_record = self
                .get_versioned_value::<NodeOperatorRecord>(make_node_operator_record_key(node_operator_id).as_str(), versions_in_registry.end_bound)?;
            let node_provider_id: PrincipalId = node_operator_record.node_provider_principal_id.try_into().unwrap();

            if let Entry::Vacant(entry) = node_operator_rewardable_count.entry(node_operator_id) {
                entry.insert(node_operator_record.rewardable_nodes);
            }
            let node_type = self.estimate_node_type(node_operator_rewardable_count.get_mut(&node_operator_id));
            let dc_id = node_operator_record.dc_id;
            let data_center_record = self.get_versioned_value::<DataCenterRecord>(&make_data_center_record_key(&dc_id), versions_in_registry.end_bound)?;
            let region = data_center_record.region;

            rewardable_nodes_per_provider.entry(node_provider_id).or_default().push(RewardableNode {
                node_id,
                node_type,
                dc_id,
                region,
                // TODO: map registry version to timestamp when registry mapping available
                rewardable_from: DayUTC::from(start_ts),
                rewardable_to: DayUTC::from(end_ts),
            })
        }
        
        Ok(rewardable_nodes_per_provider)
    }
}
