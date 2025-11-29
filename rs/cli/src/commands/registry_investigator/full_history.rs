use std::hash::{DefaultHasher, Hash, Hasher};

use crate::commands::registry_investigator::AuthRequirement;
use crate::exe::ExecutableCommand;
use crate::exe::args::GlobalArgs;
use candid::Principal;
use clap::{Args, ValueEnum};
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
use ic_types::PrincipalId;
use log::info;
use prost::Message;

#[derive(Args, Debug)]
pub struct FullHistory {
    #[clap(long)]
    key_type: KeyType,

    #[clap(long)]
    key_value: Option<String>,
}

impl ExecutableCommand for FullHistory {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let local_registry = ctx.local_registry()?;

        let latest_version = local_registry.get_latest_version();

        info!("Latest version known to the local registry: {latest_version}");

        let full_key = self.full_record_key();

        info!("Will attempt to make full history of key: {full_key}");

        let mut chain = vec![];
        let mut last_hash = 0;

        for v in 0..=latest_version.get() {
            let record_at_version = local_registry.get_versioned_value(&full_key, v.into());

            let record = match record_at_version {
                Ok(v) => v,
                Err(e) => return Err(anyhow::anyhow!("Received error at version {v}: {e:?}")),
            };

            let mut hasher = DefaultHasher::new();
            record.value.hash(&mut hasher);
            let hash = hasher.finish();

            if hash == last_hash {
                continue;
            }

            last_hash = hash;
            chain.push(record);
        }

        info!("Found {} state transitions for queried key", chain.len());

        self.display_chain(chain)
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        match self.key_type {
            KeyType::SubnetList | KeyType::NodeRewardsTable | KeyType::BlessedReplicaVersions => return,
            KeyType::ApiBoundaryNode
            | KeyType::Node
            | KeyType::NodeOperator
            | KeyType::ReplicaVersion
            | KeyType::HostOsVersion
            | KeyType::Subnet
            | KeyType::DataCenter
                if self.key_value.is_none() => {}
            _ => return,
        }

        cmd.error(
            clap::error::ErrorKind::InvalidValue,
            format!("Value is mandatory with submitted key type"),
        )
        .exit();
    }
}

impl FullHistory {
    fn key_type_to_prefix(&self) -> String {
        match self.key_type {
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

    fn full_record_key(&self) -> String {
        match self.key_type {
            KeyType::SubnetList | KeyType::NodeRewardsTable | KeyType::BlessedReplicaVersions => return self.key_type_to_prefix(),
            _ => {}
        }

        let prefix = self.key_type_to_prefix();
        let value = self.key_value.clone().unwrap();

        format!("{prefix}{value}")
    }

    fn content_to_value(&self, content: RegistryVersionedRecord<Vec<u8>>) -> anyhow::Result<String> {
        let content = match content.value {
            None => return Ok("Deletion Marker".to_string()),
            Some(v) => v,
        };

        let decoded_record = match self.key_type {
            KeyType::SubnetList => SubnetListRecord::decode(content.as_slice()).map(DecodedRecord::SubnetList),
            KeyType::NodeRewardsTable => NodeRewardsTable::decode(content.as_slice()).map(DecodedRecord::NodeRewardsTable),
            KeyType::BlessedReplicaVersions => BlessedReplicaVersions::decode(content.as_slice()).map(DecodedRecord::BlessedReplicaVersions),
            KeyType::ApiBoundaryNode => ApiBoundaryNodeRecord::decode(content.as_slice()).map(DecodedRecord::ApiBoundaryNode),
            KeyType::Node => NodeRecord::decode(content.as_slice()).map(DecodedRecord::Node),
            KeyType::NodeOperator => NodeOperatorRecord::decode(content.as_slice()).map(DecodedRecord::NodeOperator),
            KeyType::ReplicaVersion => ReplicaVersionRecord::decode(content.as_slice()).map(DecodedRecord::ReplicaVersion),
            KeyType::HostOsVersion => HostosVersionRecord::decode(content.as_slice()).map(DecodedRecord::HostOsVersion),
            KeyType::Subnet => SubnetRecord::decode(content.as_slice()).map(DecodedRecord::Subnet),
            KeyType::DataCenter => DataCenterRecord::decode(content.as_slice()).map(DecodedRecord::DataCenter),
        }
        .map_err(anyhow::Error::from)?;

        serialize_decoded_record(decoded_record)
    }

    fn display_chain(&self, chain: Vec<RegistryVersionedRecord<Vec<u8>>>) -> anyhow::Result<()> {
        for content_at_version in chain {
            println!("Version: {}", content_at_version.version);
            println!("Value:\n{}", self.content_to_value(content_at_version)?);
            println!();
        }

        Ok(())
    }
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

enum DecodedRecord {
    DeletionMarker,
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
        DecodedRecord::DeletionMarker => return Ok("deletion marker".to_string()),
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
