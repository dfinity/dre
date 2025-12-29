use clap::Args;

use crate::commands::registry::helpers::filters::Filter;
use crate::commands::registry::helpers::versions::VersionRange;
use crate::commands::registry::helpers::dump::{get_sorted_versions_from_local};
use crate::commands::registry::helpers::versions::VersionFillMode;
use crate::commands::registry::helpers::writer::create_writer;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use ic_registry_common_proto::pb::local_store::v1::MutationType;
use ic_registry_common_proto::pb::local_store::v1::ChangelogEntry;
use std::path::PathBuf;
use serde::Serialize;
use log::info;
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use prost::Message;

#[derive(Args, Debug)]
#[clap(about = "Show history for a version range")]
pub struct History {
    #[clap(index = 1, allow_hyphen_values = true, help = format!("Version number or negative index

- No argument will show history from latest-10 to latest
{}

Examples:
  -5              # Show history of latest-5 to latest
  -1              # Show history of latest
  55400           # Show history from 55400 to latest
", VersionRange::get_help_text()))]
    pub version_1: Option<i64>,

    #[clap(index = 2, allow_hyphen_values = true, help = "Version number or negative index

See [VERSION_1] for more information.
Only supported in combination with [VERSION_1].

Examples for combination with [VERSION_1]:
  -5 -2           # Show history of latest-5 to latest-2
  55400 55450     # Show history from 55400 to 55450
    ")]
    pub version_2: Option<i64>,

    #[clap(short = 'o', long, help = "Output file (default is stdout)")]
    pub output: Option<PathBuf>,

    #[clap(short = 'f', long, help = Filter::get_help_text())]
    pub filter: Vec<Filter>,
}

impl ExecutableCommand for History {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // Ensure local registry is initialized/synced
        let _ = ctx.load_registry().await;

        // Get sorted versions
        let (versions_in_registry, entries_sorted) = get_sorted_versions_from_local(&ctx).await?;

        // Create version range
        let version_range = VersionRange::create_from_args(self.version_1, self.version_2, VersionFillMode::ToEnd, &versions_in_registry)?;
        info!("Selected version range: {:?}", version_range);

        // Build flat list of records
        let entries_map: std::collections::HashMap<u64, ChangelogEntry> = entries_sorted.into_iter().collect();
        let selected_versions: Vec<u64> = (version_range.get_from().unwrap()..=version_range.get_to().unwrap()).collect();
        let flattened_version_records: Vec<FlattenedVersionRecord> = FlattenedVersionRecord::create_from_selected_version(&selected_versions, &entries_map);
        // let out: Vec<VersionRecord> = flatten_version_records(&selected_versions, &entries_map);

        // Write to file or stdout
        let writer = create_writer(&self.output)?;
        serde_json::to_writer_pretty(writer, &flattened_version_records)?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct FlattenedVersionRecord {
    version: u64,
    key: String,
    value: Value,
}

impl FlattenedVersionRecord {
    fn create_from_selected_version(selected_versions: &Vec<u64>, entries_map: &std::collections::HashMap<u64, ChangelogEntry>) -> Vec<FlattenedVersionRecord>  {
        let mut flattened_version_records: Vec<FlattenedVersionRecord> = Vec::new();

        for v in selected_versions {
            if let Some(entry) = entries_map.get(v) {
                for km in entry.key_mutations.iter() {
                    let value_json = match km.mutation_type() {
                        MutationType::Unset => Value::Null,
                        MutationType::Set => decode_value_to_json(&km.key, &km.value),
                        _ => Value::Null,
                    };
                    flattened_version_records.push(FlattenedVersionRecord {
                        version: *v,
                        key: km.key.clone(),
                        value: value_json,
                    });
                }
            }
        }
        flattened_version_records
        }
    }

/// Best-effort decode of registry value bytes into JSON. Falls back to hex when unknown.
/// This can be extended to specific types in the future, if needed
fn decode_value_to_json(key: &str, bytes: &[u8]) -> Value {
    // Known families where we can decode via protobuf types pulled from workspace crates.
    // Use key prefixes to route decoding. Keep minimal and practical.
    if key.starts_with(ic_registry_keys::DATA_CENTER_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::dc::v1::DataCenterRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key.starts_with(ic_registry_keys::NODE_OPERATOR_RECORD_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::node_operator::v1::NodeOperatorRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key.starts_with(ic_registry_keys::NODE_RECORD_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::node::v1::NodeRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key.starts_with(ic_registry_keys::SUBNET_RECORD_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::subnet::v1::SubnetRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key.starts_with(ic_registry_keys::REPLICA_VERSION_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::replica_version::v1::ReplicaVersionRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key.starts_with(ic_registry_keys::HOSTOS_VERSION_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::hostos_version::v1::HostosVersionRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key == ic_registry_keys::NODE_REWARDS_TABLE_KEY {
        if let Ok(rec) = ic_protobuf::registry::node_rewards::v2::NodeRewardsTable::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key.starts_with(ic_registry_keys::API_BOUNDARY_NODE_RECORD_KEY_PREFIX) {
        if let Ok(rec) = ic_protobuf::registry::api_boundary_node::v1::ApiBoundaryNodeRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key == "unassigned_nodes_config" {
        if let Ok(rec) = ic_protobuf::registry::unassigned_nodes_config::v1::UnassignedNodesConfigRecord::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    } else if key == "blessed_replica_versions" {
        if let Ok(rec) = ic_protobuf::registry::replica_version::v1::BlessedReplicaVersions::decode(bytes) {
            return normalize_protobuf_json(serde_json::to_value(&rec).unwrap_or(Value::Null));
        }
    }

    // Fallback: base64 for compactness
    let s = BASE64.encode(bytes);
    if bytes.len() <= 29 {
        if let Ok(p) = ic_types::PrincipalId::try_from(bytes.to_vec()) {
            return serde_json::json!({ "bytes_base64": s, "principal": p.to_string() });
        }
    }
    serde_json::json!({ "bytes_base64": s })
}

/// Recursively convert protobuf-derived JSON so byte arrays become base64 strings
fn normalize_protobuf_json(mut v: Value) -> Value {
    match &mut v {
        Value::Array(arr) => {
            for e in arr.iter_mut() {
                *e = normalize_protobuf_json(std::mem::take(e));
            }
        }
        Value::Object(map) => {
            for (_, vv) in map.iter_mut() {
                *vv = normalize_protobuf_json(std::mem::take(vv));
            }
        }
        Value::Number(_) | Value::String(_) | Value::Bool(_) | Value::Null => {}
    }

    // Replace array of small integers (likely bytes) with base64 when appropriate
    if let Value::Array(arr) = &v {
        if !arr.is_empty()
            && arr
                .iter()
                .all(|x| matches!(x, Value::Number(n) if n.as_u64().is_some() && n.as_u64().unwrap() <= 255))
        {
            let mut buf = Vec::with_capacity(arr.len());
            for x in arr {
                if let Value::Number(n) = x {
                    buf.push(n.as_u64().unwrap() as u8);
                }
            }
            let s = BASE64.encode(&buf);
            if buf.len() <= 29 {
                if let Ok(p) = ic_types::PrincipalId::try_from(buf) {
                    return serde_json::json!({"bytes_base64": s, "principal": p.to_string()});
                }
            }
            return serde_json::json!({ "bytes_base64": s });
        }
    }
    v
}
