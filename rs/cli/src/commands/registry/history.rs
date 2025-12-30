use crate::commands::registry::helpers::dump::get_sorted_versions_from_local;
use crate::commands::registry::helpers::filters::Filter;
use crate::commands::registry::helpers::versions::{VersionFillMode, VersionRange};
use crate::commands::registry::helpers::writer::Writer;
use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::Args;
use ic_registry_common_proto::pb::local_store::v1::{ChangelogEntry, MutationType};
use log::info;
use prost::Message;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

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

    #[clap(
        index = 2,
        allow_hyphen_values = true,
        help = "Version number or negative index

See [VERSION_1] for more information.
Only supported in combination with [VERSION_1].

Examples for combination with [VERSION_1]:
  -5 -2           # Show history of latest-5 to latest-2
  55400 55450     # Show history from 55400 to 55450
    "
    )]
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
        let _ = ctx.registry().await;

        // Get sorted versions
        let (versions_sorted, entries_sorted) = get_sorted_versions_from_local(&ctx).await?;

        // Create version range
        let version_range = VersionRange::create_from_args(self.version_1, self.version_2, VersionFillMode::ToEnd, &versions_sorted)?;
        info!("Selected version range {:?}", version_range);

        // Build flat list of records
        let entries_map: std::collections::HashMap<u64, ChangelogEntry> = entries_sorted.into_iter().collect();
        let selected_versions: Vec<u64> = (version_range.get_from()..=version_range.get_to()).collect();
        let flattened_version_records: Vec<FlattenedVersionRecord> =
            FlattenedVersionRecord::create_from_selected_version(&selected_versions, &entries_map);

        // Apply filters
        let mut flattened_version_records_json = serde_json::to_value(flattened_version_records)?;
        self.filter.iter().for_each(|filter| {
            let _ = filter.filter_json_value(&mut flattened_version_records_json);
        });

        // Write to file or stdout
        let mut writer = Writer::new(&self.output, false)?;
        writer.write_line(&serde_json::to_string_pretty(&flattened_version_records_json)?)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct FlattenedVersionRecord {
    version: u64,
    key: String,
    value: Value,
}

impl FlattenedVersionRecord {
    fn create_from_selected_version(
        selected_versions: &Vec<u64>,
        entries_map: &std::collections::HashMap<u64, ChangelogEntry>,
    ) -> Vec<FlattenedVersionRecord> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ic_protobuf::registry::api_boundary_node::v1::ApiBoundaryNodeRecord;
    use ic_protobuf::registry::dc::v1::DataCenterRecord;
    use ic_protobuf::registry::hostos_version::v1::HostosVersionRecord;
    use ic_protobuf::registry::node::v1::NodeRecord;
    use ic_protobuf::registry::node_operator::v1::NodeOperatorRecord;
    use ic_protobuf::registry::node_rewards::v2::NodeRewardsTable;
    use ic_protobuf::registry::replica_version::v1::{BlessedReplicaVersions, ReplicaVersionRecord};
    use ic_protobuf::registry::subnet::v1::SubnetRecord;
    use ic_protobuf::registry::unassigned_nodes_config::v1::UnassignedNodesConfigRecord;
    use std::collections::HashMap;

    #[test]
    fn test_create_from_selected_version() {
        // Test data
        struct TestCase {
            description: String,
            input: (Vec<u64>, HashMap<u64, ChangelogEntry>),
            output: Vec<FlattenedVersionRecord>,
        }

        let test_cases = vec![
            TestCase {
                description: "empty entries map".to_string(),
                input: (vec![1, 2, 3], HashMap::new()),
                output: vec![],
            },
            TestCase {
                description: "empty selected versions".to_string(),
                input: (vec![], std::collections::HashMap::from([(1, ChangelogEntry { key_mutations: vec![] })])),
                output: vec![],
            },
            TestCase {
                description: "single version with single key mutation (Set)".to_string(),
                input: (
                    vec![1],
                    std::collections::HashMap::from([(
                        1,
                        ChangelogEntry {
                            key_mutations: vec![ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                key: "test_key".to_string(),
                                value: b"test_value_too_long_to_be_principal_id".to_vec(),
                                mutation_type: MutationType::Set as i32,
                            }],
                        },
                    )]),
                ),
                output: vec![FlattenedVersionRecord {
                    version: 1,
                    key: "test_key".to_string(),
                    value: serde_json::json!({"bytes_base64": "dGVzdF92YWx1ZV90b29fbG9uZ190b19iZV9wcmluY2lwYWxfaWQ="}),
                }],
            },
            TestCase {
                description: "single version with single key mutation (Unset)".to_string(),
                input: (
                    vec![1],
                    std::collections::HashMap::from([(
                        1,
                        ChangelogEntry {
                            key_mutations: vec![ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                key: "test_key".to_string(),
                                value: vec![],
                                mutation_type: MutationType::Unset as i32,
                            }],
                        },
                    )]),
                ),
                output: vec![FlattenedVersionRecord {
                    version: 1,
                    key: "test_key".to_string(),
                    value: serde_json::Value::Null,
                }],
            },
            TestCase {
                description: "multiple versions with multiple key mutations".to_string(),
                input: (
                    vec![1, 2],
                    std::collections::HashMap::from([
                        (
                            1,
                            ChangelogEntry {
                                key_mutations: vec![
                                    ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                        key: "key1".to_string(),
                                        value: vec![
                                            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
                                            29, 30, 31,
                                        ],
                                        mutation_type: MutationType::Set as i32,
                                    },
                                    ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                        key: "key2".to_string(),
                                        value: vec![
                                            100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120,
                                            121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131,
                                        ],
                                        mutation_type: MutationType::Set as i32,
                                    },
                                ],
                            },
                        ),
                        (
                            2,
                            ChangelogEntry {
                                key_mutations: vec![ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                    key: "key3".to_string(),
                                    value: vec![
                                        200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221,
                                        222, 223, 224, 225, 226, 227, 228, 229, 230, 231,
                                    ],
                                    mutation_type: MutationType::Set as i32,
                                }],
                            },
                        ),
                    ]),
                ),
                output: vec![
                    FlattenedVersionRecord {
                        version: 1,
                        key: "key1".to_string(),
                        value: serde_json::json!({"bytes_base64": "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8="}),
                    },
                    FlattenedVersionRecord {
                        version: 1,
                        key: "key2".to_string(),
                        value: serde_json::json!({"bytes_base64": "ZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXp7fH1+f4CBgoM="}),
                    },
                    FlattenedVersionRecord {
                        version: 2,
                        key: "key3".to_string(),
                        value: serde_json::json!({"bytes_base64": "yMnKy8zNzs/Q0dLT1NXW19jZ2tvc3d7f4OHi4+Tl5uc="}),
                    },
                ],
            },
            TestCase {
                description: "version not in entries map is skipped".to_string(),
                input: (
                    vec![1, 2, 3],
                    std::collections::HashMap::from([
                        (
                            1,
                            ChangelogEntry {
                                key_mutations: vec![ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                    key: "key1".to_string(),
                                    value: vec![
                                        10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200, 210, 220, 230,
                                        240, 250, 1, 2, 3, 4, 5,
                                    ],
                                    mutation_type: MutationType::Set as i32,
                                }],
                            },
                        ),
                        (
                            3,
                            ChangelogEntry {
                                key_mutations: vec![ic_registry_common_proto::pb::local_store::v1::KeyMutation {
                                    key: "key3".to_string(),
                                    value: vec![
                                        5, 15, 25, 35, 45, 55, 65, 75, 85, 95, 105, 115, 125, 135, 145, 155, 165, 175, 185, 195, 205, 215, 225, 235,
                                        245, 10, 20, 30, 40, 50,
                                    ],
                                    mutation_type: MutationType::Set as i32,
                                }],
                            },
                        ),
                    ]),
                ),
                output: vec![
                    FlattenedVersionRecord {
                        version: 1,
                        key: "key1".to_string(),
                        value: serde_json::json!({"bytes_base64": "ChQeKDI8RlBaZG54goyWoKq0vsjS3Obw+gECAwQF"}),
                    },
                    FlattenedVersionRecord {
                        version: 3,
                        key: "key3".to_string(),
                        value: serde_json::json!({"bytes_base64": "BQ8ZIy03QUtVX2lzfYeRm6WvucPN1+Hr9QoUHigy"}),
                    },
                ],
            },
            TestCase {
                description: "version with empty key_mutations".to_string(),
                input: (vec![1], std::collections::HashMap::from([(1, ChangelogEntry { key_mutations: vec![] })])),
                output: vec![],
            },
        ];

        for test_case in test_cases {
            let result = FlattenedVersionRecord::create_from_selected_version(&test_case.input.0, &test_case.input.1);
            assert_eq!(result, test_case.output, "{}", test_case.description);
        }
    }

    #[test]
    fn test_decode_value_to_json() {
        struct TestCase {
            description: String,
            input: (String, Vec<u8>),
            output: Value,
        }

        let test_cases = vec![
            TestCase {
                description: "data center record".to_string(),
                input: (
                    format!("{}test_dc_id", ic_registry_keys::DATA_CENTER_KEY_PREFIX),
                    DataCenterRecord {
                        id: "test_dc_id".to_string(),
                        region: "test_continent,test_country,test_area".to_string(),
                        owner: "test_owner".to_string(),
                        gps: None,
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "id": "test_dc_id",
                    "region": "test_continent,test_country,test_area",
                    "owner": "test_owner",
                    "gps": null
                }),
            },
            TestCase {
                description: "node operator record".to_string(),
                input: (
                    format!("{}test_no_id", ic_registry_keys::NODE_OPERATOR_RECORD_KEY_PREFIX),
                    NodeOperatorRecord {
                        dc_id: "test_dc".to_string(),
                        node_allowance: 10,
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "node_operator_principal_id": [],
                    "node_allowance": 10,
                    "node_provider_principal_id": [],
                    "dc_id": "test_dc",
                    "rewardable_nodes": {},
                    "ipv6": null,
                    "max_rewardable_nodes": {}
                }),
            },
            TestCase {
                description: "replica version record".to_string(),
                input: (
                    format!("{}test_replica_version", ic_registry_keys::REPLICA_VERSION_KEY_PREFIX),
                    ReplicaVersionRecord {
                        release_package_urls: vec!["https://example.com/release.tar.gz".to_string()],
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "release_package_sha256_hex": "",
                    "release_package_urls": ["https://example.com/release.tar.gz"],
                    "guest_launch_measurements": null
                }),
            },
            TestCase {
                description: "hostos version record".to_string(),
                input: (
                    format!("{}test_hostos_version", ic_registry_keys::HOSTOS_VERSION_KEY_PREFIX),
                    HostosVersionRecord {
                        release_package_urls: vec!["https://example.com/hostos.tar.gz".to_string()],
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "release_package_urls": ["https://example.com/hostos.tar.gz"],
                    "release_package_sha256_hex": "",
                    "hostos_version_id": ""
                }),
            },
            TestCase {
                description: "unassigned nodes config record".to_string(),
                input: (
                    "unassigned_nodes_config".to_string(),
                    UnassignedNodesConfigRecord {
                        replica_version: "test_replica_version".to_string(),
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "ssh_readonly_access": [],
                    "replica_version": "test_replica_version"
                }),
            },
            TestCase {
                description: "blessed replica versions".to_string(),
                input: (
                    "blessed_replica_versions".to_string(),
                    BlessedReplicaVersions {
                        blessed_version_ids: vec!["version1".to_string(), "version2".to_string()],
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "blessed_version_ids": ["version1", "version2"]
                }),
            },
            TestCase {
                description: "node record".to_string(),
                input: (
                    format!("{}test_node_id", ic_registry_keys::NODE_RECORD_KEY_PREFIX),
                    NodeRecord {
                        node_operator_id: vec![1, 2, 3, 4, 5],
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "xnet": null,
                    "http": null,
                    "node_operator_id": {"bytes_base64": "AQIDBAU=", "principal": "i4fzt-5abai-bqibi"},
                    "chip_id": null,
                    "hostos_version_id": null,
                    "public_ipv4_config": null,
                    "domain": null,
                    "node_reward_type": null,
                    "ssh_node_state_write_access": []
                }),
            },
            TestCase {
                description: "subnet record".to_string(),
                input: (
                    format!("{}test_subnet_id", ic_registry_keys::SUBNET_RECORD_KEY_PREFIX),
                    SubnetRecord {
                        membership: vec![vec![1, 2, 3], vec![4, 5, 6]],
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "membership": [{"bytes_base64": "AQID", "principal": "kw6ia-hibai-bq"}, {"bytes_base64": "BAUG", "principal": "nrocb-pqeau-da"}],
                    "max_ingress_bytes_per_message": 0,
                    "unit_delay_millis": 0,
                    "initial_notary_delay_millis": 0,
                    "replica_version_id": "",
                    "dkg_interval_length": 0,
                    "start_as_nns": false,
                    "subnet_type": 0,
                    "dkg_dealings_per_block": 0,
                    "is_halted": false,
                    "max_ingress_messages_per_block": 0,
                    "max_block_payload_size": 0,
                    "features": null,
                    "max_number_of_canisters": 0,
                    "ssh_readonly_access": [],
                    "ssh_backup_access": [],
                    "halt_at_cup_height": false,
                    "chain_key_config": null,
                    "canister_cycles_cost_schedule": 0
                }),
            },
            TestCase {
                description: "node rewards table".to_string(),
                input: (
                    ic_registry_keys::NODE_REWARDS_TABLE_KEY.to_string(),
                    NodeRewardsTable {
                        table: std::collections::BTreeMap::new(),
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "table": {}
                }),
            },
            TestCase {
                description: "api boundary node record".to_string(),
                input: (
                    format!("{}test_api_bn_id", ic_registry_keys::API_BOUNDARY_NODE_RECORD_KEY_PREFIX),
                    ApiBoundaryNodeRecord {
                        version: "test_version".to_string(),
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                output: serde_json::json!({
                    "version": "test_version"
                }),
            },
            TestCase {
                description: "unknown key falls back to base64".to_string(),
                input: ("unknown_key_prefix".to_string(), b"some_arbitrary_bytes_that_are_longer_than_29".to_vec()),
                output: serde_json::json!({
                    "bytes_base64": "c29tZV9hcmJpdHJhcnlfYnl0ZXNfdGhhdF9hcmVfbG9uZ2VyX3RoYW5fMjk="
                }),
            },
        ];

        for test_case in test_cases {
            let result = decode_value_to_json(&test_case.input.0, &test_case.input.1);
            assert_eq!(result, test_case.output, "{}", test_case.description);
        }
    }
}
