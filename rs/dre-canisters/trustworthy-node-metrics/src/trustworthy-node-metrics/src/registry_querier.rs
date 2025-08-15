use std::{
    collections::{btree_map::Entry, BTreeMap},
    rc::Rc,
    str::FromStr,
};

use ic_protobuf::registry::{
    dc::v1::DataCenterRecord, node::v1::NodeRecord, node_operator::v1::NodeOperatorRecord, node_rewards::v2::NodeRewardsTable,
};
use ic_registry_keys::{make_data_center_record_key, make_node_operator_record_key, NODE_RECORD_KEY_PREFIX, NODE_REWARDS_TABLE_KEY};
use ic_types::{PrincipalId, RegistryVersion};
use itertools::Itertools;
use node_provider_rewards_lib::v1_types::Node;

use crate::{chrono_utils::DateTimeRange, local_registry::LocalRegistry};

pub struct RegistryQuerier {
    pub local_registry: Rc<LocalRegistry>,
}

impl RegistryQuerier {
    pub fn nodes_in_period(&self, _period: &DateTimeRange) -> Vec<Node> {
        let mut nodes = BTreeMap::new();
        let mut rewardable_nodes = BTreeMap::new();
        let versions = vec![self.local_registry.get_latest_version().get()];

        ic_cdk::println!("versions in range: {:?}", versions);

        for version in versions {
            let registry_version = RegistryVersion::from(version);
            let nodes_in_version = self
                .local_registry
                .get_family_entries_of_version::<NodeRecord>(NODE_RECORD_KEY_PREFIX, registry_version)
                .unwrap();

            for (p, (_, node_record)) in nodes_in_version {
                let principal = PrincipalId::from_str(p.as_str()).unwrap();

                if let Entry::Vacant(node) = nodes.entry(principal) {
                    let node_operator_id: PrincipalId = node_record.node_operator_id.try_into().unwrap();
                    let key = make_node_operator_record_key(node_operator_id);
                    let node_operator_record = self
                        .local_registry
                        .get_versioned_value::<NodeOperatorRecord>(key.as_str(), registry_version)
                        .unwrap();

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

    pub async fn get_rewards_table(&self) -> anyhow::Result<NodeRewardsTable> {
        ic_cdk::println!("Fetching NodeRewardsTable from registry canister");
        let (rewards_table, _): (NodeRewardsTable, _) = ic_nns_common::registry::get_value(NODE_REWARDS_TABLE_KEY.as_bytes(), None).await?;
        Ok(rewards_table)
    }
}
