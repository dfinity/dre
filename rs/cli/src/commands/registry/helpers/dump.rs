use crate::ctx::DreContext;
use ic_canisters::IcAgentCanisterClient;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_management_backend::health::HealthStatusQuerier;
use ic_management_backend::lazy_registry::LazyRegistry;
use ic_management_types::{HealthStatus, Network};
use ic_protobuf::registry::dc::v1::DataCenterRecord;
use ic_protobuf::registry::hostos_version::v1::HostosVersionRecord;
use ic_protobuf::registry::node::v1::{ConnectionEndpoint, IPv4InterfaceConfig, NodeRewardType};
use ic_protobuf::registry::replica_version::v1::ReplicaVersionRecord;
use ic_protobuf::registry::subnet::v1::{ChainKeyConfig, SubnetFeatures};
use ic_protobuf::registry::unassigned_nodes_config::v1::UnassignedNodesConfigRecord;
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use icp_ledger::AccountIdentifier;
use indexmap::IndexMap;
use itertools::Itertools;
use log::{info, warn};
use prost::Message;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    net::Ipv6Addr,
    str::FromStr,
    sync::Arc,
};

#[derive(Debug, Serialize)]
pub(crate) struct RegistryDump {
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

pub(crate) async fn get_dump_from_registry(ctx: DreContext) -> anyhow::Result<RegistryDump> {
    let local_registry_result = ctx.get_registry();
    if let Err(e) = local_registry_result {
        return Err(anyhow::anyhow!("Failed to get registry: {:?}", e));
    }
    let local_registry = local_registry_result.unwrap();

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
        node_providers: get_node_providers(&local_registry, ctx.network(), ctx.is_offline()).await?,
    })
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

pub(crate) async fn get_sorted_versions_from_local(
    ctx: &DreContext,
) -> anyhow::Result<(Vec<u64>, Vec<(u64, ic_registry_common_proto::pb::local_store::v1::ChangelogEntry)>)> {
    let base_dirs = get_dirs_from_ctx(ctx)?;

    let entries = load_first_available_entries(&base_dirs)?;
    let mut entries_sorted = entries;
    entries_sorted.sort_by_key(|(v, _)| *v);
    let versions_sorted: Vec<u64> = entries_sorted.iter().map(|(v, _)| *v).collect();

    if versions_sorted.is_empty() {
        anyhow::bail!("No registry versions available");
    }

    info!(
        "Available versions in local store: {} to {} ({} versions)",
        versions_sorted.first().unwrap(),
        versions_sorted.last().unwrap(),
        versions_sorted.len()
    );

    Ok((versions_sorted, entries_sorted))
}

fn get_dirs_from_ctx(ctx: &DreContext) -> anyhow::Result<Vec<std::path::PathBuf>> {
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

async fn get_node_providers(local_registry: &Arc<dyn LazyRegistry>, network: &Network, offline: bool) -> anyhow::Result<Vec<NodeProvider>> {
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
    let reg_node_providers = local_registry
        .operators()
        .await?
        .values()
        .map(|operator| operator.provider.clone())
        .collect_vec();

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

pub(crate) fn load_first_available_entries(
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
