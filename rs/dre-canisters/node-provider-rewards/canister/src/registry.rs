use ic_base_types::{NodeId, PrincipalId, RegistryVersion, SubnetId};
use ic_interfaces_registry::RegistryValue;
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::node::v1::NodeRecord;
use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
use ic_protobuf::registry::subnet::v1::{SubnetListRecord, SubnetRecord};
use ic_registry_canister_client::{
    CanisterRegistryClient, RegistryDataStableMemory, StableCanisterRegistryClient, StorableRegistryKey, StorableRegistryValue,
};
use ic_registry_keys::{
    make_subnet_list_record_key, DATA_CENTER_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX, NODE_REWARDS_TABLE_KEY,
    SUBNET_RECORD_KEY_PREFIX,
};
use ic_types::registry::RegistryClientError;
use indexmap::IndexMap;
use rewards_calculation::rewards_calculator_results::DayUTC;
use rewards_calculation::types::{NodeType, ProviderRewardableNodes, Region, RewardableNode, TimestampNanos};
use std::collections::{BTreeMap, HashMap};
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

struct RegistryVersionBounds {
    start_bound: RegistryVersion,
    end_bound: RegistryVersion,
}

struct NodeOperatorData {
    node_provider_id: PrincipalId,
    dc_id: String,
    region: Region,
    rewardable_nodes_count: BTreeMap<NodeType, u32>,
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

    /// Nodes In Range
    ///
    /// This function retrieves all nodes present in the registry within a specified registry version range.
    /// The results include the node ID, the node record, and the version bounds indicating the period during which the node was present in the registry.
    /// Nodes deleted before `registry_version_range.start_bound` are not included in the results.
    /// Nodes are ordered by their registration time, with the earliest registered node appearing first.
    fn nodes_in_range(
        &self,
        registry_version_range: RegistryVersionBounds,
    ) -> Result<IndexMap<NodeId, (NodeRecord, RegistryVersionBounds)>, RegistryClientError> {
        let version_end = registry_version_range.end_bound.get();
        let version_start = registry_version_range.start_bound.get();

        let key_prefix = NodeRecord::KEY_PREFIX;
        let prefix_length = key_prefix.len();
        let start_range = StorableRegistryKey::new(key_prefix.to_string(), Default::default());

        let mut registered_between_versions = IndexMap::new();
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
                        registered_between_versions.shift_remove(&node_key.key);
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

        Ok(registered_between_versions
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
            .collect())
    }

    fn node_operators_data(&self, end_bound: RegistryVersion) -> HashMap<PrincipalId, NodeOperatorData> {
        let node_operators = self
            .get_family_entries_of_version::<NodeOperatorRecord>(end_bound)
            .into_iter()
            .map(|(_, (_, node_operator_record))| {
                (
                    PrincipalId::try_from(node_operator_record.node_operator_principal_id.clone()).expect("Failed to parse PrincipalId"),
                    node_operator_record,
                )
            })
            .collect::<HashMap<_, _>>();
        let data_centers = self.get_family_entries_of_version::<DataCenterRecord>(end_bound);

        node_operators
            .into_iter()
            .map(|(node_operator_id, node_operator_record)| {
                let node_provider_id: PrincipalId = node_operator_record
                    .node_provider_principal_id
                    .try_into()
                    .expect("Failed to parse PrincipalId");
                let dc_id = node_operator_record.dc_id.clone();
                let (_, data_center_record) = data_centers.get(&dc_id).expect("Failed to find dc_id");
                let region = Region(data_center_record.region.clone());

                let mut rewardable_nodes_count = BTreeMap::new();
                for (node_type, count) in node_operator_record.rewardable_nodes.into_iter() {
                    let node_type = NodeType(node_type.clone());
                    rewardable_nodes_count.insert(node_type, count);
                }
                let node_operator_data = NodeOperatorData {
                    node_provider_id,
                    dc_id,
                    region,
                    rewardable_nodes_count,
                };
                (node_operator_id, node_operator_data)
            })
            .collect()
    }

    pub fn get_rewardable_nodes_per_provider(
        &self,
        start_ts: TimestampNanos,
        end_ts: TimestampNanos,
    ) -> Result<BTreeMap<PrincipalId, ProviderRewardableNodes>, RegistryClientError> {
        let mut rewardable_nodes_per_provider: BTreeMap<_, ProviderRewardableNodes> = BTreeMap::new();
        let mut node_operator_rewardable_count: HashMap<_, (BTreeMap<_, _>, HashMap<_, (NodeType, RegistryVersion)>)> = HashMap::new();

        // TODO: Replace to cover all the versions in the reward period once the registry supports it.
        // https://github.com/dfinity/ic/pull/4450
        let end_bound = self.store.get_latest_version();
        let start_bound = RegistryVersion::from(end_bound.get() - 100);

        let nodes_in_range = self.nodes_in_range(RegistryVersionBounds { start_bound, end_bound })?;
        let node_operators_data = self.node_operators_data(end_bound);

        for (node_id, (node_record, versions_in_registry)) in nodes_in_range.into_iter().rev() {
            let node_operator_id: PrincipalId = node_record.node_operator_id.try_into().unwrap();
            let NodeOperatorData {
                node_provider_id,
                dc_id,
                region,
                rewardable_nodes_count,
            } = if let Some(node_operator_data) = node_operators_data.get(&node_operator_id) {
                node_operator_data
            } else {
                // Reward only node operators that are registered in the registry at the end of the period
                continue;
            };

            let provider_rewardable_nodes = rewardable_nodes_per_provider.entry(*node_provider_id).or_insert(ProviderRewardableNodes {
                provider_id: *node_provider_id,
                ..Default::default()
            });
            let (node_type_count, type_assigned) = node_operator_rewardable_count.entry(node_operator_id).or_insert_with(|| {
                for (node_type, count) in rewardable_nodes_count.iter() {
                    // Rewardable nodes per provider is needed to compute base rewards
                    provider_rewardable_nodes
                        .rewardable_nodes_count
                        .entry((region.clone(), node_type.clone()))
                        .and_modify(|c| *c += count)
                        .or_insert(*count);
                }
                (rewardable_nodes_count.clone(), HashMap::new())
            });

            let ip_addr = node_record.http.expect("missing ipv6 address").ip_addr;
            let node_type = if let Some((node_type, version_from)) = type_assigned.get_mut(&ip_addr) {
                // In this case the node has been redeployed in the period, it gets the same node type
                *version_from = versions_in_registry.start_bound;
                Ok(node_type.clone())
            } else if !node_type_count.is_empty() {
                self.assign_node_type(ip_addr.clone(), &versions_in_registry, node_type_count, type_assigned)
            } else {
                // No rewardable nodes found for this node operator
                Err("No rewardable nodes left for this node operator".to_string())
            };
            let node_type = match node_type {
                Ok(node_type) => node_type,
                Err(_) => continue,
            };

            provider_rewardable_nodes.rewardable_nodes.push(RewardableNode {
                node_id,
                node_type,
                dc_id: dc_id.clone(),
                region: region.clone(),
                // TODO: ts corresponding to versions_in_registry.start_bound/end_bound
                rewardable_from: DayUTC::from(start_ts),
                rewardable_to: DayUTC::from(end_ts),
            });
        }
        Ok(rewardable_nodes_per_provider)
    }

    fn assign_node_type(
        &self,
        ip_addr: String,
        versions_in_registry: &RegistryVersionBounds,
        node_type_count: &mut BTreeMap<NodeType, u32>,
        type_assigned: &mut HashMap<String, (NodeType, RegistryVersion)>,
    ) -> Result<NodeType, String> {
        let (node_type, mut count) = loop {
            let (k, count) = match node_type_count.pop_first() {
                Some(kv) => kv,
                None => {
                    // In this case the node could have changed its IP address
                    // Checking if exists a node registered later and take its type
                    if let Some(node_type) = type_assigned
                        .values()
                        .find(|(_, version_from)| version_from > &versions_in_registry.end_bound)
                        .map(|(node_type, _)| node_type.clone())
                    {
                        type_assigned.insert(ip_addr.clone(), (node_type.clone(), versions_in_registry.start_bound));
                        return Ok(node_type);
                    } else {
                        return Err("No rewardable nodes left for this node operator".to_string())?;
                    }
                }
            };

            if count != 0 {
                break (k, count);
            }
        };
        count = count.saturating_sub(1);
        if count != 0 {
            node_type_count.insert(node_type.clone(), count);
        }
        Ok(node_type)
    }
}
