use std::collections::VecDeque;

use crate::exe::impl_executable_command_for_enums;
use candid::Principal;
use clap::{Args, ValueEnum};
use full_history::FullHistory;
use ic_interfaces_registry::{RegistryClient, RegistryVersionedRecord};
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord,
    dc::v1::DataCenterRecord,
    hostos_version::v1::HostosVersionRecord,
    node::v1::NodeRecord,
    node_operator::v1::NodeOperatorRecord,
    node_rewards::v2::NodeRewardsTable,
    replica_version::v1::{BlessedReplicaVersions, ReplicaVersionRecord},
    subnet::v1::{SubnetListRecord, SubnetRecord},
};
use ic_registry_keys::{
    API_BOUNDARY_NODE_RECORD_KEY_PREFIX, DATA_CENTER_KEY_PREFIX, HOSTOS_VERSION_KEY_PREFIX, NODE_OPERATOR_RECORD_KEY_PREFIX, NODE_RECORD_KEY_PREFIX,
    NODE_REWARDS_TABLE_KEY, REPLICA_VERSION_KEY_PREFIX, SUBNET_RECORD_KEY_PREFIX,
};
use ic_registry_local_registry::LocalRegistry;
use ic_types::RegistryVersion;

mod full_history;

#[derive(Args, Debug)]
pub struct RegistryInvestigator {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! {
    RegistryInvestigator, FullHistory
}

/// Supported key types
#[derive(Debug, Clone, ValueEnum)]
enum KeyType {
    SubnetList,

    NodeRewardsTable,

    BlessedReplicaVersions,

    #[clap(aliases = ["api-bn"])]
    ApiBoundaryNode,

    #[clap(aliases = ["n"])]
    Node,

    #[clap(aliases = ["no"])]
    NodeOperator,

    ReplicaVersion,

    HostOsVersion,

    #[clap(aliases = ["s"])]
    Subnet,

    #[clap(aliases = ["dc"])]
    DataCenter,
}

impl KeyType {
    pub fn to_registry_prefix(&self) -> String {
        match &self {
            KeyType::SubnetList => "subnet_list",
            KeyType::NodeRewardsTable => NODE_REWARDS_TABLE_KEY,
            KeyType::BlessedReplicaVersions => "blessed_replica_versions",
            KeyType::ApiBoundaryNode => API_BOUNDARY_NODE_RECORD_KEY_PREFIX,
            KeyType::Node => NODE_RECORD_KEY_PREFIX,
            KeyType::NodeOperator => NODE_OPERATOR_RECORD_KEY_PREFIX,
            KeyType::ReplicaVersion => REPLICA_VERSION_KEY_PREFIX,
            KeyType::HostOsVersion => HOSTOS_VERSION_KEY_PREFIX,
            KeyType::Subnet => SUBNET_RECORD_KEY_PREFIX,
            KeyType::DataCenter => DATA_CENTER_KEY_PREFIX,
        }
        .to_string()
    }
}

// Some handy tools for the registry investigations
struct RegistryDiagnoser {
    registry: LocalRegistry,
}

impl RegistryDiagnoser {
    fn fetch_all_changes_for_key_up_to_version(
        &self,
        key: &str,
        version: RegistryVersion,
    ) -> anyhow::Result<VecDeque<RegistryVersionedRecord<Vec<u8>>>> {
        let mut version = version;
        let mut chain = VecDeque::new();

        while version.get() != 0 {
            let record_at_version = self.registry.get_versioned_value(key, version);

            let record = match record_at_version {
                Ok(v) => v,
                Err(e) => return Err(anyhow::anyhow!("Received error at version {version}: {e:?}")),
            };

            if record.version.get() == 0 {
                break;
            }

            version = record.version.decrement();

            chain.push_front(record);
        }

        Ok(chain)
    }
}

enum DecodedRecord {
    SubnetList(SubnetListRecord),
    NodeRewardsTable(NodeRewardsTable),
    BlessedReplicaVersions(BlessedReplicaVersions),
    ApiBoundaryNode(ApiBoundaryNodeRecord),
    Node(NodeRecord),
    NodeOperator(NodeOperatorRecord),
    ReplicaVersion(ReplicaVersionRecord),
    HostOsVersion(HostosVersionRecord),
    Subnet(SubnetRecord),
    DataCenter(DataCenterRecord),
}

fn serialize_decoded_record(decoded_record: DecodedRecord) -> anyhow::Result<String> {
    let raw_record = match decoded_record {
        DecodedRecord::SubnetList(subnet_list_record) => serde_json::to_value(subnet_list_record),
        DecodedRecord::NodeRewardsTable(node_rewards_table) => serde_json::to_value(node_rewards_table),
        DecodedRecord::BlessedReplicaVersions(blessed_replica_versions) => serde_json::to_value(blessed_replica_versions),
        DecodedRecord::ApiBoundaryNode(api_boundary_node_record) => serde_json::to_value(api_boundary_node_record),
        DecodedRecord::Node(node_record) => serde_json::to_value(node_record),
        DecodedRecord::NodeOperator(node_operator_record) => serde_json::to_value(node_operator_record),
        DecodedRecord::ReplicaVersion(replica_version_record) => serde_json::to_value(replica_version_record),
        DecodedRecord::HostOsVersion(hostos_version_record) => serde_json::to_value(hostos_version_record),
        DecodedRecord::Subnet(subnet_list_record) => serde_json::to_value(subnet_list_record),
        DecodedRecord::DataCenter(data_center_record) => serde_json::to_value(data_center_record),
    }
    .map_err(anyhow::Error::from)?;

    let raw_record = fixup_ids(raw_record);

    serde_json::to_string_pretty(&raw_record).map_err(anyhow::Error::from)
}

fn fixup_ids(mut value: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Array(arr) = &value {
        // Try to convert the JSON array of numbers into a Vec<u8>
        let byte_vec: Option<Vec<u8>> = arr
            .iter()
            // Check if all elements are numbers that can fit in a u8
            .all(|v| v.is_u64() && v.as_u64().unwrap() <= 255)
            .then(|| arr.iter().map(|v| v.as_u64().unwrap() as u8).collect());

        if let Some(bytes) = byte_vec {
            return serde_json::Value::String(Principal::from_slice(&bytes).to_string());
        }
    }

    if let serde_json::Value::Object(map) = &mut value {
        for value in map.values_mut() {
            *value = fixup_ids(std::mem::take(value));
        }
    } else if let serde_json::Value::Array(arr) = &mut value {
        for item in arr {
            *item = fixup_ids(std::mem::take(item));
        }
    }
    value
}
