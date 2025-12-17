use crate::ctx::DreContext;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ic_canisters::IcAgentCanisterClient;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_management_backend::{health::HealthStatusQuerier, lazy_registry::LazyRegistry};
use ic_management_types::{HealthStatus, Network};
use ic_protobuf::registry::node::v1::NodeRewardType;
use ic_protobuf::registry::{
    dc::v1::DataCenterRecord,
    hostos_version::v1::HostosVersionRecord,
    node::v1::{ConnectionEndpoint, IPv4InterfaceConfig},
    replica_version::v1::ReplicaVersionRecord,
    subnet::v1::{ChainKeyConfig, SubnetFeatures},
    unassigned_nodes_config::v1::UnassignedNodesConfigRecord,
};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use icp_ledger::AccountIdentifier;
use indexmap::IndexMap;
use itertools::Itertools;
use log::{info, warn};
use prost::Message;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::iter::{IntoIterator, Iterator};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::Ipv6Addr,
    str::FromStr,
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct Filter {
    key: String,
    value: Value,
    comparison: Comparison,
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Define the regex pattern for `key comparison value` with optional spaces
        let re = Regex::new(r"^\s*(\w+)\s*\b(.+?)\b\s*(.*)$").unwrap();

        // Capture key, comparison, and value
        if let Some(captures) = re.captures(s) {
            let key = captures[1].to_string();
            let comparison_str = &captures[2];
            let value_str = &captures[3];

            let comparison = Comparison::from_str(comparison_str)?;

            let value = serde_json::from_str(value_str).unwrap_or_else(|_| serde_json::Value::String(value_str.to_string()));

            Ok(Self { key, value, comparison })
        } else {
            anyhow::bail!(
                "Expected format: `key comparison value` (spaces around the comparison are optional, supported comparison: = != > < >= <= re contains startswith endswith), found {}",
                s
            );
        }
    }
}

/// Validate a range of version indices.
///
/// This function validates that the range has at most 2 elements and reorders
/// the range to ensure increasing order. If an empty vector is passed, it defaults to [-10].
/// The function will fail if:
/// - 0 is passed in the range
/// - Positive version numbers are mixed with negative indices
///
/// # Arguments
/// * `range` - The range vector to validate and normalize
///
/// # Returns
/// * `Ok(Vec<i64>)` - The normalized range (reordered if needed, or [-10] if empty)
/// * `Err` - If validation fails (e.g., if 0 is in the range, mixing positive/negative, or more than 2 elements)
pub(crate) fn validate_range(range: &[i64]) -> anyhow::Result<Vec<i64>> {
    if range.is_empty() {
        return Ok(vec![-10]);
    }
    if range.len() > 2 {
        anyhow::bail!("Range accepts at most 2 arguments (FROM and TO), got {}", range.len());
    }

    // Fail if 0 is passed
    if range.contains(&0) {
        anyhow::bail!("Range cannot contain 0");
    }

    // Fail if mixing positive version numbers and negative indices
    let has_positive = range.iter().any(|&x| x > 0);
    let has_negative = range.iter().any(|&x| x < 0);
    if has_positive && has_negative {
        anyhow::bail!("Cannot mix positive version numbers and negative indices in range");
    }

    // Reorder if needed to ensure increasing order
    let normalized = if range.len() == 2 && range[0] > range[1] {
        vec![range[1], range[0]]
    } else {
        range.to_vec()
    };

    Ok(normalized)
}

pub(crate) async fn get_sorted_versions(
    ctx: &DreContext,
) -> anyhow::Result<(Vec<u64>, Vec<(u64, ic_registry_common_proto::pb::local_store::v1::ChangelogEntry)>)> {
    // Ensure local registry is initialized/synced
    let _ = ctx.registry_with_version(None, false).await;
    let base_dirs = local_registry_dirs_for_ctx(ctx)?;
    let entries = load_first_available_entries(&base_dirs)?;
    let mut entries_sorted = entries;
    entries_sorted.sort_by_key(|(v, _)| *v);
    let versions_sorted: Vec<u64> = entries_sorted.iter().map(|(v, _)| *v).collect();

    if versions_sorted.is_empty() {
        anyhow::bail!("No registry versions available");
    }

    Ok((versions_sorted, entries_sorted))
}

pub(crate) fn create_writer(output: &Option<std::path::PathBuf>) -> anyhow::Result<Box<dyn std::io::Write>> {
    match output {
        Some(path) => {
            let file = fs_err::File::create(path)?;
            info!("Writing to file: {:?}", file.path().canonicalize()?);
            Ok(Box::new(std::io::BufWriter::new(file)))
        }
        None => Ok(Box::new(std::io::stdout())),
    }
}

// Helper: collect candidate base dirs
fn local_registry_dirs_for_ctx(ctx: &DreContext) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut base_dirs = vec![
        dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Couldn't find cache dir for dre-store"))?
            .join("dre-store")
            .join("local_registry")
            .join(&ctx.network().name),
    ];
    if let Ok(override_dir) = std::env::var("DRE_LOCAL_REGISTRY_DIR_OVERRIDE") {
        base_dirs.insert(0, std::path::PathBuf::from(override_dir));
    }
    base_dirs.push(std::path::PathBuf::from("/tmp/dre-test-store/local_registry").join(&ctx.network().name));
    Ok(base_dirs)
}

fn load_first_available_entries(
    base_dirs: &[std::path::PathBuf],
) -> anyhow::Result<Vec<(u64, ic_registry_common_proto::pb::local_store::v1::ChangelogEntry)>> {
    use ic_registry_common_proto::pb::local_store::v1::ChangelogEntry as PbChangelogEntry;
    use std::ffi::OsStr;

    let mut entries: Vec<(u64, PbChangelogEntry)> = Vec::new();
    for base_dir in base_dirs.iter() {
        let mut local: Vec<(u64, PbChangelogEntry)> = Vec::new();
        collect_pb_files(base_dir, &mut |path| {
            if path.extension() == Some(OsStr::new("pb")) {
                if let Some(v) = extract_version_from_registry_path(base_dir, path) {
                    let bytes = std::fs::read(path).unwrap_or_else(|_| panic!("Failed reading {}", path.display()));
                    let entry = PbChangelogEntry::decode(bytes.as_slice()).unwrap_or_else(|_| panic!("Failed decoding {}", path.display()));
                    local.push((v, entry));
                }
            }
        })?;
        if !local.is_empty() {
            entries = local;
            break;
        }
    }
    if entries.is_empty() {
        anyhow::bail!("No registry versions found in local store");
    }
    Ok(entries)
}

// Slicing semantics:
// - End-inclusive (both from and to are included)
// - Positive numbers are treated as actual version numbers (0 is not supported)
// - Negative numbers are treated as indices (Python-style from the end, -1 is last)
// - If a single positive number is provided, return versions from 1 to that number
// - Versions are returned as increasing vector
pub(crate) fn select_versions(versions: Option<Vec<i64>>, versions_sorted: &[u64]) -> anyhow::Result<Vec<u64>> {
    let n = versions_sorted.len();
    let args = versions.unwrap_or_default();
    let (from_opt, to_opt): (Option<i64>, Option<i64>) = match args.as_slice() {
        [] => (None, None),
        [from] if *from > 0 => {
            // Single positive number: return from 1 to that number
            (Some(1), Some(*from))
        }
        [from] => (Some(*from), None),
        [from, to] if from == to => {
            // Same number twice: return just that single version
            (Some(*from), Some(*to))
        }
        [from, to] => (Some(*from), Some(*to)),
        _ => unreachable!(),
    };
    if n == 0 {
        return Ok(vec![]);
    }

    // Convert input to array indices
    let to_index = |idx: i64| -> anyhow::Result<usize> {
        match idx {
            i if i < 0 => {
                // Negative: treat as index from the end (Python-style)
                let j = (n as i64) + i;
                if j < 0 {
                    anyhow::bail!("Index {} is out of range for {} available versions", idx, n);
                }
                Ok(j.clamp(0, n as i64) as usize)
            }
            i if i > 0 => {
                // Positive: treat as actual version number, find its position
                let version = i as u64;
                versions_sorted
                    .binary_search(&version)
                    .map_err(|_| anyhow::anyhow!("Version {} not found in available versions", version))
            }
            0 => {
                // 0 is not supported as a version number
                Err(anyhow::anyhow!("Version 0 is not supported"))
            }
            _ => unreachable!(),
        }
    };

    let mut a = match from_opt {
        Some(idx) => to_index(idx)?,
        None => 0,
    };
    let mut b = match to_opt {
        Some(idx) => to_index(idx)?,
        None => n - 1, // Last index for end-inclusive
    };

    // Reorder if needed (swap a and b if a > b)
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }

    // End-inclusive range: use ..= to include both a and b
    // Result is already sorted since versions_sorted is sorted
    let result = versions_sorted[a..=b].to_vec();

    Ok(result)
}

pub(crate) fn flatten_version_records(
    selected_versions: &[u64],
    entries_map: &std::collections::HashMap<u64, ic_registry_common_proto::pb::local_store::v1::ChangelogEntry>,
) -> Vec<VersionRecord> {
    use ic_registry_common_proto::pb::local_store::v1::MutationType;
    let mut out: Vec<VersionRecord> = Vec::new();
    for v in selected_versions {
        if let Some(entry) = entries_map.get(v) {
            for km in entry.key_mutations.iter() {
                let value_json = match km.mutation_type() {
                    MutationType::Unset => Value::Null,
                    MutationType::Set => decode_value_to_json(&km.key, &km.value),
                    _ => Value::Null,
                };
                out.push(VersionRecord {
                    version: *v,
                    key: km.key.clone(),
                    value: value_json,
                });
            }
        }
    }
    out
}

pub struct Registry {}

impl Registry {
    async fn get_registry(&self, ctx: DreContext, height: Option<u64>, offline: bool) -> anyhow::Result<RegistryDump> {
        let local_registry = ctx.registry_with_version(height, offline).await;

        let elected_guest_os_versions = get_elected_guest_os_versions(&local_registry)?;
        let elected_host_os_versions = get_elected_host_os_versions(&local_registry)?;

        let mut node_operators = get_node_operators(&local_registry, ctx.network()).await?;

        let dcs = local_registry.get_datacenters()?;

        let (subnets, nodes) = get_subnets_and_nodes(&local_registry, &node_operators, ctx.health_client()).await?;

        let unassigned_nodes_config = local_registry.get_unassigned_nodes()?;

        // Calculate number of rewardable nodes for node operators
        for node_operator in node_operators.values_mut() {
            let mut nodes_by_health = IndexMap::new();
            for node_details in nodes.iter().filter(|n| n.node_operator_id == node_operator.node_operator_principal_id) {
                let node_id = node_details.node_id;
                let health = node_details.status.to_string();
                let nodes = nodes_by_health.entry(health).or_insert_with(Vec::new);
                nodes.push(node_id);
            }
            node_operator.computed.nodes_health = nodes_by_health;
            node_operator.computed.total_up_nodes = nodes
                .iter()
                .filter(|n| {
                    n.node_operator_id == node_operator.node_operator_principal_id
                        && (n.status == HealthStatus::Healthy || n.status == HealthStatus::Degraded)
                })
                .count() as u32;
        }
        let node_rewards_table = get_node_rewards_table(&local_registry, ctx.network());

        let api_bns = get_api_boundary_nodes(&local_registry)?;

        Ok(RegistryDump {
            elected_guest_os_versions,
            elected_host_os_versions,
            nodes,
            subnets,
            unassigned_nodes_config,
            dcs,
            node_operators: node_operators.values().cloned().collect_vec(),
            node_rewards_table,
            api_bns,
            node_providers: get_node_providers(&local_registry, ctx.network(), ctx.is_offline(), height.is_none()).await?,
        })
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct VersionRecord {
    version: u64,
    key: String,
    value: Value,
}

fn extract_version_from_registry_path(base_dir: &std::path::Path, full_path: &std::path::Path) -> Option<u64> {
    // Registry path ends with .../<10 hex>/<2 hex>/<2 hex>/<5 hex>.pb
    // We reconstruct the hex by concatenating the four segments (without slashes) and parse as hex u64.
    let rel = full_path.strip_prefix(base_dir).ok()?;
    let parts: Vec<_> = rel.iter().map(|s| s.to_string_lossy()).collect();

    if parts.len() == 1 {
        let hex = parts[0].trim_end_matches(".pb");
        return u64::from_str_radix(hex, 16).ok();
    }

    if parts.len() < 4 {
        return None;
    }
    let last = parts[parts.len() - 1].trim_end_matches(".pb");
    let seg3 = &parts[parts.len() - 2];
    let seg2 = &parts[parts.len() - 3];
    let seg1 = &parts[parts.len() - 4];
    let hex = format!("{}{}{}{}", seg1, seg2, seg3, last);
    u64::from_str_radix(&hex, 16).ok()
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

fn collect_pb_files<F: FnMut(&std::path::Path)>(base: &std::path::Path, visitor: &mut F) -> anyhow::Result<()> {
    if !base.exists() {
        return Ok(());
    }
    for entry in fs_err::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_pb_files(&path, visitor)?;
        } else {
            visitor(&path);
        }
    }
    Ok(())
}

fn get_elected_guest_os_versions(local_registry: &Arc<dyn LazyRegistry>) -> anyhow::Result<Vec<ReplicaVersionRecord>> {
    local_registry.elected_guestos_records()
}

fn get_elected_host_os_versions(local_registry: &Arc<dyn LazyRegistry>) -> anyhow::Result<Vec<HostosVersionRecord>> {
    local_registry.elected_hostos_records()
}

async fn get_node_operators(local_registry: &Arc<dyn LazyRegistry>, network: &Network) -> anyhow::Result<IndexMap<PrincipalId, NodeOperator>> {
    let all_nodes = local_registry.nodes().await?;
    let operators = local_registry
        .operators()
        .await
        .map_err(|e| anyhow::anyhow!("Couldn't get node operators: {:?}", e))?;
    let node_operators = operators
        .iter()
        .map(|(k, record)| {
            let node_provider_name = record.provider.name.as_ref().map_or_else(String::default, |v| {
                if network.is_mainnet() && v.is_empty() {
                    panic!("Node provider name should not be empty for mainnet")
                }
                v.to_string()
            });
            let nodes_in_subnets = all_nodes
                .iter()
                .filter(|(_, value)| value.operator.principal == record.principal && value.subnet_id.is_some())
                .count() as u64;
            let nodes_in_registry = all_nodes.iter().filter(|(_, value)| value.operator.principal == record.principal).count() as u64;

            (
                record.principal,
                NodeOperator {
                    node_operator_principal_id: *k,
                    node_provider_principal_id: record.provider.principal,
                    dc_id: record.datacenter.as_ref().map(|d| d.name.to_owned()).unwrap_or_default(),
                    rewardable_nodes: record.rewardable_nodes.clone(),
                    max_rewardable_nodes: record.max_rewardable_nodes.clone(),
                    node_allowance: record.node_allowance,
                    ipv6: Some(record.ipv6.to_string()),
                    computed: NodeOperatorComputed {
                        node_provider_name,
                        total_up_nodes: 0,
                        nodes_health: Default::default(),
                        nodes_in_subnets,
                        nodes_in_registry,
                    },
                },
            )
        })
        .collect::<IndexMap<_, _>>();
    Ok(node_operators)
}

async fn get_subnets_and_nodes(
    local_registry: &Arc<dyn LazyRegistry>,
    node_operators: &IndexMap<PrincipalId, NodeOperator>,
    health_client: Arc<dyn HealthStatusQuerier>,
) -> anyhow::Result<(Vec<SubnetRecord>, Vec<NodeDetails>)> {
    let subnets = local_registry.subnets().await?;
    let subnets = subnets
        .iter()
        .map(|(subnet_id, record)| SubnetRecord {
            subnet_id: *subnet_id,
            membership: record.nodes.iter().map(|n| n.principal.to_string()).collect(),
            nodes: Default::default(),
            max_ingress_bytes_per_message: record.max_ingress_bytes_per_message,
            max_ingress_messages_per_block: record.max_ingress_messages_per_block,
            max_block_payload_size: record.max_block_payload_size,
            unit_delay_millis: record.unit_delay_millis,
            initial_notary_delay_millis: record.initial_notary_delay_millis,
            replica_version_id: record.replica_version.clone(),
            dkg_interval_length: record.dkg_interval_length,
            start_as_nns: record.start_as_nns,
            subnet_type: record.subnet_type,
            features: record.features.unwrap_or_default(),
            max_number_of_canisters: record.max_number_of_canisters,
            ssh_readonly_access: record.ssh_readonly_access.clone(),
            ssh_backup_access: record.ssh_backup_access.clone(),
            dkg_dealings_per_block: record.dkg_dealings_per_block,
            is_halted: record.is_halted,
            halt_at_cup_height: record.halt_at_cup_height,
            chain_key_config: record.chain_key_config.clone(),
        })
        .collect::<Vec<_>>();
    let nodes = _get_nodes(local_registry, node_operators, &subnets, health_client).await?;
    let subnets = subnets
        .into_iter()
        .map(|subnet| {
            let nodes = nodes
                .iter()
                .filter(|n| subnet.membership.contains(&n.node_id.to_string()))
                .cloned()
                .map(|n| (n.node_id, n))
                .collect::<IndexMap<_, _>>();
            SubnetRecord { nodes, ..subnet }
        })
        .collect();
    Ok((subnets, nodes))
}

async fn _get_nodes(
    local_registry: &Arc<dyn LazyRegistry>,
    node_operators: &IndexMap<PrincipalId, NodeOperator>,
    subnets: &[SubnetRecord],
    health_client: Arc<dyn HealthStatusQuerier>,
) -> anyhow::Result<Vec<NodeDetails>> {
    let nodes_health = health_client.nodes().await?;
    let nodes = local_registry.nodes().await.map_err(|e| anyhow::anyhow!("Couldn't get nodes: {:?}", e))?;
    let nodes = nodes
        .iter()
        .map(|(k, record)| {
            let node_operator_id = record.operator.principal;
            let subnet = subnets.iter().find(|subnet| subnet.membership.contains(&k.to_string()));

            NodeDetails {
                node_id: *k,
                xnet: Some(ConnectionEndpoint {
                    ip_addr: record.ip_addr.unwrap_or(Ipv6Addr::LOCALHOST).to_string(),
                    port: 2497,
                }),
                http: Some(ConnectionEndpoint {
                    ip_addr: record.ip_addr.unwrap_or(Ipv6Addr::LOCALHOST).to_string(),
                    port: 8080,
                }),
                node_operator_id,
                chip_id: record.chip_id.clone(),
                hostos_version_id: Some(record.hostos_version.clone()),
                public_ipv4_config: record.public_ipv4_config.clone(),
                node_provider_id: match node_operators.get(&node_operator_id) {
                    Some(no) => no.node_provider_principal_id,
                    None => PrincipalId::new_anonymous(),
                },
                subnet_id: subnet.map(|subnet| subnet.subnet_id),
                dc_id: match node_operators.get(&node_operator_id) {
                    Some(no) => no.dc_id.clone(),
                    None => "".to_string(),
                },
                status: nodes_health.get(k).unwrap_or(&ic_management_types::HealthStatus::Unknown).clone(),
                node_reward_type: record.node_reward_type.unwrap_or(NodeRewardType::Unspecified).to_string(),
                dc_owner: record.operator.datacenter.clone().map(|dc| dc.owner.name).unwrap_or_default(),
                guestos_version_id: subnet.map(|sr| sr.replica_version_id.to_string()),
                country: record.operator.datacenter.clone().map(|dc| dc.country).unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    Ok(nodes)
}

fn get_node_rewards_table(local_registry: &Arc<dyn LazyRegistry>, network: &Network) -> NodeRewardsTableFlattened {
    let rewards_table_bytes = local_registry.get_node_rewards_table();

    let mut rewards_table = match rewards_table_bytes {
        Ok(r) => r,
        Err(_) => {
            if network.is_mainnet() {
                panic!("Failed to get Node Rewards Table for mainnet")
            } else {
                warn!("Failed to get Node Rewards Table for {}", network.name);
                IndexMap::new()
            }
        }
    };

    let table = match rewards_table.first_entry() {
        Some(f) => f.get().table.clone(),
        None => {
            warn!("Failed to get Node Rewards Table for {}", network.name);
            BTreeMap::new()
        }
    };

    NodeRewardsTableFlattened {
        table: table
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    NodeRewardRatesFlattened {
                        rates: v
                            .rates
                            .iter()
                            .map(|(rate_key, rate_val)| {
                                (
                                    rate_key.clone(),
                                    NodeRewardRateFlattened {
                                        xdr_permyriad_per_node_per_month: rate_val.xdr_permyriad_per_node_per_month,
                                        reward_coefficient_percent: rate_val.reward_coefficient_percent,
                                    },
                                )
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

fn get_api_boundary_nodes(local_registry: &Arc<dyn LazyRegistry>) -> anyhow::Result<Vec<ApiBoundaryNodeDetails>> {
    let api_bns = local_registry
        .get_api_boundary_nodes()
        .map_err(|e| anyhow::anyhow!("Couldn't get api boundary nodes: {:?}", e))?
        .into_iter()
        .map(|(k, record)| {
            let principal = PrincipalId::from_str(k.as_str()).expect("Couldn't parse principal id");
            ApiBoundaryNodeDetails {
                principal,
                version: record.version,
            }
        })
        .collect();

    Ok(api_bns)
}

async fn get_node_providers(
    local_registry: &Arc<dyn LazyRegistry>,
    network: &Network,
    offline: bool,
    latest_height: bool,
) -> anyhow::Result<Vec<NodeProvider>> {
    let all_nodes = local_registry.nodes().await?;

    // Get the node providers from the node operator records, and from the governance canister, and merge them
    let nns_urls = network.get_nns_urls();
    let url = nns_urls.first().ok_or(anyhow::anyhow!("No NNS URLs provided"))?.to_owned();
    let canister_client = IcAgentCanisterClient::from_anonymous(url)?;
    let gov = GovernanceCanisterWrapper::from(canister_client);
    let gov_node_providers: HashMap<PrincipalId, String> = if !offline {
        gov.get_node_providers()
            .await?
            .iter()
            .map(|p| {
                (
                    p.id.unwrap_or(PrincipalId::new_anonymous()),
                    match &p.reward_account {
                        Some(account) => AccountIdentifier::from_slice(&account.hash).unwrap().to_string(),
                        None => "".to_string(),
                    },
                )
            })
            .collect()
    } else {
        HashMap::new()
    };
    let mut reg_node_providers = local_registry
        .operators()
        .await?
        .values()
        .map(|operator| operator.provider.clone())
        .collect_vec();
    let reg_provider_ids = reg_node_providers.iter().map(|provider| provider.principal).collect::<HashSet<_>>();

    // Governance canister doesn't have the mechanism to retrieve node providers on a certain height
    // meaning that merging the lists on arbitrary heights wouldn't make sense.
    if latest_height {
        for principal in gov_node_providers.keys() {
            if !reg_provider_ids.contains(principal) {
                reg_node_providers.push(ic_management_types::Provider {
                    principal: *principal,
                    name: None,
                    website: None,
                });
            }
        }
    }
    let reg_node_providers = reg_node_providers
        .into_iter()
        .sorted_by_key(|provider| provider.principal)
        .dedup_by(|x, y| x.principal == y.principal)
        .collect_vec();

    Ok(reg_node_providers
        .iter()
        .map(|provider| {
            let provider_nodes = all_nodes.values().filter(|node| node.operator.provider.principal == provider.principal);

            NodeProvider {
                principal: provider.principal,
                reward_account: gov_node_providers.get(&provider.principal).cloned().unwrap_or_default(),
                total_nodes: provider_nodes.clone().count(),
                nodes_in_subnet: provider_nodes.clone().filter(|node| node.subnet_id.is_some()).count(),
                nodes_per_dc: provider_nodes
                    .map(|node| match &node.operator.datacenter {
                        Some(dc) => dc.name.clone(),
                        None => "Unknown".to_string(),
                    })
                    .counts_by(|dc_name| dc_name)
                    .into_iter()
                    .collect(),
                name: provider.name.clone().unwrap_or("Unknown".to_string()),
            }
        })
        .collect())
}

#[derive(Debug, Serialize)]
struct RegistryDump {
    subnets: Vec<SubnetRecord>,
    nodes: Vec<NodeDetails>,
    unassigned_nodes_config: Option<UnassignedNodesConfigRecord>,
    dcs: Vec<DataCenterRecord>,
    node_operators: Vec<NodeOperator>,
    node_rewards_table: NodeRewardsTableFlattened,
    api_bns: Vec<ApiBoundaryNodeDetails>,
    elected_guest_os_versions: Vec<ReplicaVersionRecord>,
    elected_host_os_versions: Vec<HostosVersionRecord>,
    node_providers: Vec<NodeProvider>,
}

#[derive(Clone, Debug, Serialize)]
struct ApiBoundaryNodeDetails {
    principal: PrincipalId,
    version: String,
}

#[derive(Debug, Serialize, Clone)]
struct NodeDetails {
    node_id: PrincipalId,
    xnet: Option<ConnectionEndpoint>,
    http: Option<ConnectionEndpoint>,
    node_operator_id: PrincipalId,
    chip_id: Option<Vec<u8>>,
    hostos_version_id: Option<String>,
    public_ipv4_config: Option<IPv4InterfaceConfig>,
    subnet_id: Option<PrincipalId>,
    dc_id: String,
    node_provider_id: PrincipalId,
    status: HealthStatus,
    node_reward_type: String,
    dc_owner: String,
    guestos_version_id: Option<String>,
    country: String,
}

/// User-friendly representation of a SubnetRecord. For instance,
/// the `membership` field is a `Vec<String>` to pretty-print the node IDs.
#[derive(Debug, Default, Serialize, Clone)]
struct SubnetRecord {
    subnet_id: PrincipalId,
    membership: Vec<String>,
    nodes: IndexMap<PrincipalId, NodeDetails>,
    max_ingress_bytes_per_message: u64,
    max_ingress_messages_per_block: u64,
    max_block_payload_size: u64,
    unit_delay_millis: u64,
    initial_notary_delay_millis: u64,
    replica_version_id: String,
    dkg_interval_length: u64,
    dkg_dealings_per_block: u64,
    start_as_nns: bool,
    subnet_type: SubnetType,
    features: SubnetFeatures,
    max_number_of_canisters: u64,
    ssh_readonly_access: Vec<String>,
    ssh_backup_access: Vec<String>,
    is_halted: bool,
    halt_at_cup_height: bool,
    chain_key_config: Option<ChainKeyConfig>,
}

#[derive(Clone, Debug, Serialize)]
struct NodeOperator {
    node_operator_principal_id: PrincipalId,
    node_provider_principal_id: PrincipalId,
    dc_id: String,
    rewardable_nodes: BTreeMap<String, u32>,
    max_rewardable_nodes: BTreeMap<String, u32>,
    node_allowance: u64,
    ipv6: Option<String>,
    computed: NodeOperatorComputed,
}

#[derive(Clone, Debug, Serialize)]
struct NodeOperatorComputed {
    node_provider_name: String,
    total_up_nodes: u32,
    nodes_health: IndexMap<String, Vec<PrincipalId>>,
    nodes_in_subnets: u64,
    nodes_in_registry: u64,
}

// We re-create the rewards structs here in order to convert the output of get-rewards-table into the format
// that can also be parsed by propose-to-update-node-rewards-table.
// This is a bit of a hack, but it's the easiest way to get the desired output.
// A more proper way would be to adjust the upstream structs to flatten the "rates" and "table" fields
// directly, but this breaks some of the candid encoding and decoding and also some of the tests.
// Make sure to keep these structs in sync with the upstream ones.
#[derive(serde::Serialize, PartialEq, ::prost::Message)]
pub struct NodeRewardRateFlattened {
    #[prost(uint64, tag = "1")]
    pub xdr_permyriad_per_node_per_month: u64,
    #[prost(int32, optional, tag = "2")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_coefficient_percent: Option<i32>,
}

#[derive(serde::Serialize, PartialEq, ::prost::Message)]
pub struct NodeRewardRatesFlattened {
    #[prost(btree_map = "string, message", tag = "1")]
    #[serde(flatten)]
    pub rates: BTreeMap<String, NodeRewardRateFlattened>,
}

#[derive(serde::Serialize, PartialEq, ::prost::Message)]
pub struct NodeRewardsTableFlattened {
    #[prost(btree_map = "string, message", tag = "1")]
    #[serde(flatten)]
    pub table: BTreeMap<String, NodeRewardRatesFlattened>,
}

#[derive(serde::Serialize, Debug)]
struct NodeProvider {
    name: String,
    principal: PrincipalId,
    reward_account: String,
    total_nodes: usize,
    nodes_in_subnet: usize,
    nodes_per_dc: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
enum Comparison {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Regex,
    Contains,
    StartsWith,
    EndsWith,
}

impl FromStr for Comparison {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eq" | "=" | "==" => Ok(Comparison::Equal),
            "ne" | "!=" => Ok(Comparison::NotEqual),
            "gt" | ">" => Ok(Comparison::GreaterThan),
            "lt" | "<" => Ok(Comparison::LessThan),
            "ge" | ">=" => Ok(Comparison::GreaterThanOrEqual),
            "le" | "<=" => Ok(Comparison::LessThanOrEqual),
            "regex" | "re" | "matches" | "=~" => Ok(Comparison::Regex),
            "contains" => Ok(Comparison::Contains),
            "startswith" => Ok(Comparison::StartsWith),
            "endswith" => Ok(Comparison::EndsWith),
            _ => anyhow::bail!("Invalid comparison operator: {}", s),
        }
    }
}

impl Comparison {
    fn matches(&self, value: &Value, other: &Value) -> bool {
        match self {
            Comparison::Equal => value == other,
            Comparison::NotEqual => value != other,
            Comparison::GreaterThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() > b.as_f64(),
                (Value::String(a), Value::String(b)) => a > b,
                _ => false,
            },
            Comparison::LessThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() < b.as_f64(),
                (Value::String(a), Value::String(b)) => a < b,
                _ => false,
            },
            Comparison::GreaterThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() >= b.as_f64(),
                (Value::String(a), Value::String(b)) => a >= b,
                _ => false,
            },
            Comparison::LessThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() <= b.as_f64(),
                (Value::String(a), Value::String(b)) => a <= b,
                _ => false,
            },
            Comparison::Regex => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        let re = Regex::new(other).unwrap();
                        re.is_match(s)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Comparison::Contains => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other { s.contains(other) } else { false }
                } else {
                    false
                }
            }
            Comparison::StartsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other { s.starts_with(other) } else { false }
                } else {
                    false
                }
            }
            Comparison::EndsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other { s.ends_with(other) } else { false }
                } else {
                    false
                }
            }
        }
    }
}

fn filter_json_value(current: &mut Value, key: &str, value: &Value, comparison: &Comparison) -> bool {
    match current {
        Value::Object(map) => {
            // Check if current object contains key-value pair
            if let Some(v) = map.get(key) {
                return comparison.matches(v, value);
            }

            // Filter nested objects
            map.retain(|_, v| filter_json_value(v, key, value, comparison));

            // If the map is empty consider it doesn't contain the key-value
            !map.is_empty()
        }
        Value::Array(arr) => {
            // Filter entries in the array
            arr.retain_mut(|v| filter_json_value(v, key, value, comparison));

            // If the array is empty consider it doesn't contain the key-value
            !arr.is_empty()
        }
        _ => false, // Since this is a string comparison, non-object and non-array values don't match
    }
}