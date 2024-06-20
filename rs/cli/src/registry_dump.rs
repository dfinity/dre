use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use ic_base_types::{PrincipalId, RegistryVersion};
use ic_interfaces_registry::RegistryClient;
use ic_management_backend::{
    health::{HealthClient, HealthStatusQuerier},
    public_dashboard::query_ic_dashboard_list,
    registry::{local_registry_path, sync_local_store, RegistryFamilyEntries},
};
use ic_management_types::{Network, NodeProvidersResponse, Status};
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord,
    dc::v1::DataCenterRecord,
    hostos_version::v1::HostosVersionRecord,
    node::v1::{ConnectionEndpoint, IPv4InterfaceConfig, NodeRecord},
    node_operator::v1::NodeOperatorRecord,
    replica_version::v1::ReplicaVersionRecord,
    subnet::v1::{EcdsaConfig, GossipConfig as GossipConfigProto, SubnetFeatures, SubnetRecord as SubnetRecordProto},
    unassigned_nodes_config::v1::UnassignedNodesConfigRecord,
};
use ic_registry_keys::NODE_REWARDS_TABLE_KEY;
use ic_registry_local_registry::LocalRegistry;
use ic_registry_subnet_type::SubnetType;
use itertools::Itertools;
use log::{info, warn};
use registry_canister::mutations::common::decode_registry_value;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct RegistryDump {
    elected_guest_os_versions: Vec<ReplicaVersionRecord>,
    elected_host_os_versions: Vec<HostosVersionRecord>,
    nodes: Vec<NodeDetails>,
    subnets: Vec<SubnetRecord>,
    unassigned_nodes_config: Option<UnassignedNodesConfigRecord>,
    dcs: Vec<DataCenterRecord>,
    node_operators: Vec<NodeOperator>,
    node_rewards_table: NodeRewardsTableFlattened,
    api_bns: Vec<ApiBoundaryNodeDetails>,
}

async fn get_registry(path: &Option<PathBuf>, network: &Network, version: &i64) -> Result<RegistryDump, anyhow::Error> {
    if let Some(path) = path {
        std::env::set_var("LOCAL_REGISTRY_PATH", path)
    }
    sync_local_store(network).await?;

    let local_registry = LocalRegistry::new(local_registry_path(network), Duration::from_secs(10))?;

    // determine desired version
    let version = {
        if *version >= 0 {
            RegistryVersion::new(*version as u64)
        } else {
            local_registry.get_latest_version()
        }
    };

    let elected_guest_os_versions = get_elected_guest_os_versions(&local_registry, version)?;
    let elected_host_os_versions = get_elected_host_os_versions(&local_registry, version)?;

    let node_provider_names: HashMap<PrincipalId, String> = HashMap::from_iter(
        query_ic_dashboard_list::<NodeProvidersResponse>(network, "v3/node-providers")
            .await?
            .node_providers
            .iter()
            .map(|np| (np.principal_id, np.display_name.clone())),
    );
    let mut node_operators = get_node_operators(&local_registry, version, &node_provider_names)?;

    let dcs = get_data_centers(&local_registry, version)?;

    let subnets = get_subnets(&local_registry, version)?;

    let unassigned_nodes_config = match get_unassigned_nodes(&local_registry, version) {
        Ok(config) => Some(config),
        Err(RegistryDumpError::RecordNotFound) => None,
        Err(e) => return Err(e.into()),
    };

    let nodes = get_nodes(&local_registry, version, &node_operators, &subnets, network).await?;
    // Calculate number of rewardable nodes for node operators
    for node_operator in node_operators.values_mut() {
        let mut nodes_by_health = BTreeMap::new();
        for node_details in nodes.iter().filter(|n| n.node_operator_id == node_operator.node_operator_principal_id) {
            let node_id = node_details.node_id;
            let health = node_details.status.to_string();
            let nodes = nodes_by_health.entry(health).or_insert_with(Vec::new);
            nodes.push(node_id);
        }
        node_operator.nodes_health = nodes_by_health;
        node_operator.total_up_nodes = nodes
            .iter()
            .filter(|n| {
                n.node_operator_id == node_operator.node_operator_principal_id && (n.status == Status::Healthy || n.status == Status::Degraded)
            })
            .count() as u32;

        if node_operator.total_up_nodes == node_operator.rewardable_nodes.iter().map(|n| n.1).sum::<u32>() {
            node_operator.rewards_correct = true;
        }
    }
    let node_rewards_table = get_node_rewards_table(&local_registry, version, network);

    let api_bns = get_api_boundary_nodes(&local_registry, version)?;

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
    })
}

pub async fn dump_registry(
    path: &Option<PathBuf>,
    network: &Network,
    version: &i64,
    output: &Option<PathBuf>,
    incorrect_rewards_info_only: bool,
) -> Result<(), anyhow::Error> {
    let writer: Box<dyn std::io::Write> = match output {
        Some(path) => {
            let path = path.with_extension("json").canonicalize()?;
            info!("Writing to file: {:?}", path);
            Box::new(std::io::BufWriter::new(fs_err::File::create(path)?))
        }
        None => Box::new(std::io::stdout()),
    };

    if incorrect_rewards_info_only {
        let node_operators = &get_registry(path, network, version).await?;
        let node_operators = node_operators.node_operators.iter().filter(|rec| !rec.rewards_correct).collect_vec();
        serde_json::to_writer_pretty(writer, &node_operators)?;
    } else {
        serde_json::to_writer_pretty(writer, &get_registry(path, network, version).await?)?;
    }

    Ok(())
}

fn get_elected_guest_os_versions(local_registry: &LocalRegistry, version: RegistryVersion) -> Result<Vec<ReplicaVersionRecord>, RegistryDumpError> {
    let elected_versions = local_registry
        .get_family_entries_of_version::<ReplicaVersionRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get elected versions: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .collect();
    Ok(elected_versions)
}

fn get_elected_host_os_versions(local_registry: &LocalRegistry, version: RegistryVersion) -> Result<Vec<HostosVersionRecord>, RegistryDumpError> {
    let elected_versions = local_registry
        .get_family_entries_of_version::<HostosVersionRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get elected versions: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .collect();
    Ok(elected_versions)
}

async fn get_nodes(
    local_registry: &LocalRegistry,
    version: RegistryVersion,
    node_operators: &BTreeMap<PrincipalId, NodeOperator>,
    subnets: &[SubnetRecord],
    network: &Network,
) -> Result<Vec<NodeDetails>, RegistryDumpError> {
    let health_client = HealthClient::new(network.clone());
    let nodes_health = health_client.nodes().await?;

    let nodes = local_registry
        .get_family_entries_of_version::<NodeRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get nodes: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| {
            let node_operator_id = PrincipalId::try_from(&record.node_operator_id).expect("Couldn't parse principal id");
            let node_id = PrincipalId::from_str(&k).expect("Couldn't parse principal id");
            NodeDetails {
                node_id,
                xnet: record.xnet,
                http: record.http,
                node_operator_id,
                chip_id: record.chip_id,
                hostos_version_id: record.hostos_version_id,
                public_ipv4_config: record.public_ipv4_config,
                node_provider_id: node_operators
                    .get(&node_operator_id)
                    .expect("Couldn't find node provider for node operator")
                    .node_provider_principal_id,
                subnet_id: subnets
                    .iter()
                    .find(|subnet| subnet.membership.contains(&k))
                    .map(|subnet| subnet.subnet_id),
                dc_id: node_operators
                    .get(&node_operator_id)
                    .expect("Couldn't find node provider for node operator")
                    .dc_id
                    .clone(),
                status: nodes_health.get(&node_id).unwrap_or(&ic_management_types::Status::Unknown).clone(),
            }
        })
        .collect::<Vec<_>>();
    Ok(nodes)
}

fn get_subnets(local_registry: &LocalRegistry, version: RegistryVersion) -> Result<Vec<SubnetRecord>, RegistryDumpError> {
    Ok(local_registry
        .get_family_entries_of_version::<SubnetRecordProto>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get subnets: {:?}", e))?
        .into_iter()
        .map(|(subnet_id, (_, record))| SubnetRecord {
            subnet_id: PrincipalId::from_str(&subnet_id).expect("Couldn't parse principal id"),
            membership: record
                .membership
                .iter()
                .map(|n| {
                    PrincipalId::try_from(&n[..])
                        .expect("could not create PrincipalId from membership entry")
                        .to_string()
                })
                .collect(),
            nodes: Default::default(),
            max_ingress_bytes_per_message: record.max_ingress_bytes_per_message,
            max_ingress_messages_per_block: record.max_ingress_messages_per_block,
            max_block_payload_size: record.max_block_payload_size,
            unit_delay_millis: record.unit_delay_millis,
            initial_notary_delay_millis: record.initial_notary_delay_millis,
            replica_version_id: record.replica_version_id,
            dkg_interval_length: record.dkg_interval_length,
            gossip_config: record.gossip_config,
            start_as_nns: record.start_as_nns,
            subnet_type: SubnetType::try_from(record.subnet_type).unwrap(),
            max_instructions_per_message: record.max_instructions_per_message,
            max_instructions_per_round: record.max_instructions_per_round,
            max_instructions_per_install_code: record.max_instructions_per_install_code,
            features: record.features.clone().unwrap_or_default(),
            max_number_of_canisters: record.max_number_of_canisters,
            ssh_readonly_access: record.ssh_readonly_access,
            ssh_backup_access: record.ssh_backup_access,
            ecdsa_config: record.ecdsa_config,
        })
        .collect::<Vec<_>>())
}

fn get_unassigned_nodes(local_registry: &LocalRegistry, version: RegistryVersion) -> Result<UnassignedNodesConfigRecord, RegistryDumpError> {
    let unassigned_nodes_config = local_registry
        .get_family_entries_of_version::<UnassignedNodesConfigRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get unassigned nodes config: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .next()
        .ok_or(RegistryDumpError::RecordNotFound)?;
    Ok(unassigned_nodes_config)
}

fn get_data_centers(local_registry: &LocalRegistry, version: RegistryVersion) -> Result<Vec<DataCenterRecord>, RegistryDumpError> {
    Ok(local_registry
        .get_family_entries_of_version::<DataCenterRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .collect())
}

fn get_node_operators(
    local_registry: &LocalRegistry,
    version: RegistryVersion,
    node_provider_names: &HashMap<PrincipalId, String>,
) -> Result<BTreeMap<PrincipalId, NodeOperator>, RegistryDumpError> {
    let all_nodes = local_registry
        .get_family_entries_of_version::<NodeRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get nodes: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| (k, record))
        .collect::<BTreeMap<_, _>>();
    let node_operators = local_registry
        .get_family_entries_of_version::<NodeOperatorRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get node operators: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| {
            let node_operator_principal_id = PrincipalId::from_str(&k).expect("Couldn't parse principal id");
            let node_provider_name = node_provider_names
                .get(&PrincipalId::try_from(&record.node_provider_principal_id).expect("Couldn't parse principal id"))
                .expect("Couldn't find node provider name")
                .clone();
            // Find the number of nodes registered by this operator
            let operator_registered_nodes_num = all_nodes
                .iter()
                .filter(|(_, record)| {
                    PrincipalId::try_from(&record.node_operator_id).expect("Couldn't parse principal") == node_operator_principal_id
                })
                .count() as u64;
            (
                node_operator_principal_id,
                NodeOperator {
                    node_operator_principal_id,
                    node_allowance_remaining: record.node_allowance,
                    node_allowance_total: record.node_allowance + operator_registered_nodes_num,
                    node_provider_principal_id: PrincipalId::try_from(record.node_provider_principal_id).expect("Couldn't parse principal id"),
                    node_provider_name,
                    dc_id: record.dc_id,
                    rewardable_nodes: record.rewardable_nodes,
                    ipv6: record.ipv6,
                    total_up_nodes: 0,
                    nodes_health: Default::default(),
                    rewards_correct: false,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(node_operators)
}

fn get_node_rewards_table(local_registry: &LocalRegistry, version: RegistryVersion, network: &Network) -> NodeRewardsTableFlattened {
    let rewards_table_bytes = local_registry.get_value(NODE_REWARDS_TABLE_KEY, version);

    let rewards_table_bytes = match rewards_table_bytes {
        Ok(r) => match r {
            Some(r) => r,
            None => {
                if network.is_mainnet() {
                    panic!("Failed to get Node Rewards Table")
                } else {
                    warn!("Didn't find any node rewards details for network: {}", network.name);
                    vec![]
                }
            }
        },
        Err(_) => {
            if network.is_mainnet() {
                panic!("Failed to get Node Rewards Table for mainnet")
            } else {
                warn!("Failed to get Node Rewards Table for {}", network.name);
                vec![]
            }
        }
    };

    decode_registry_value::<NodeRewardsTableFlattened>(rewards_table_bytes)
}

fn get_api_boundary_nodes(local_registry: &LocalRegistry, version: RegistryVersion) -> Result<Vec<ApiBoundaryNodeDetails>, RegistryDumpError> {
    let api_bns = local_registry
        .get_family_entries_of_version::<ApiBoundaryNodeRecord>(version)
        .map_err(|e| anyhow::anyhow!("Couldn't get api boundary nodes: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| {
            let principal = PrincipalId::from_str(&k).expect("Couldn't parse principal id");
            ApiBoundaryNodeDetails {
                principal,
                version: record.version,
            }
        })
        .collect();

    Ok(api_bns)
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
    status: Status,
}

/// User-friendly representation of a SubnetRecord. For instance,
/// the `membership` field is a `Vec<String>` to pretty-print the node IDs.
#[derive(Debug, Default, Serialize, Clone)]
struct SubnetRecord {
    subnet_id: PrincipalId,
    membership: Vec<String>,
    nodes: BTreeMap<PrincipalId, NodeDetails>,
    max_ingress_bytes_per_message: u64,
    max_ingress_messages_per_block: u64,
    max_block_payload_size: u64,
    unit_delay_millis: u64,
    initial_notary_delay_millis: u64,
    replica_version_id: String,
    dkg_interval_length: u64,
    gossip_config: Option<GossipConfigProto>,
    start_as_nns: bool,
    subnet_type: SubnetType,
    max_instructions_per_message: u64,
    max_instructions_per_round: u64,
    max_instructions_per_install_code: u64,
    features: SubnetFeatures,
    max_number_of_canisters: u64,
    ssh_readonly_access: Vec<String>,
    ssh_backup_access: Vec<String>,
    ecdsa_config: Option<EcdsaConfig>,
}

#[derive(Clone, Debug, Serialize)]
struct NodeOperator {
    node_operator_principal_id: PrincipalId,
    node_allowance_remaining: u64,
    node_allowance_total: u64,
    node_provider_principal_id: PrincipalId,
    node_provider_name: String,
    dc_id: String,
    rewardable_nodes: BTreeMap<String, u32>,
    ipv6: Option<String>,
    total_up_nodes: u32,
    nodes_health: BTreeMap<String, Vec<PrincipalId>>,
    rewards_correct: bool,
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

enum RegistryDumpError {
    RegistryClientError(ic_types::registry::RegistryClientError),
    LocalRegistryError(ic_registry_local_registry::LocalRegistryError),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
    GenericError(String),
    RecordNotFound,
}

impl From<ic_types::registry::RegistryClientError> for RegistryDumpError {
    fn from(e: ic_types::registry::RegistryClientError) -> Self {
        RegistryDumpError::RegistryClientError(e)
    }
}

impl From<ic_registry_local_registry::LocalRegistryError> for RegistryDumpError {
    fn from(e: ic_registry_local_registry::LocalRegistryError) -> Self {
        RegistryDumpError::LocalRegistryError(e)
    }
}

impl From<std::io::Error> for RegistryDumpError {
    fn from(e: std::io::Error) -> Self {
        RegistryDumpError::IoError(e)
    }
}

impl From<serde_json::Error> for RegistryDumpError {
    fn from(e: serde_json::Error) -> Self {
        RegistryDumpError::SerdeError(e)
    }
}

impl From<String> for RegistryDumpError {
    fn from(e: String) -> Self {
        RegistryDumpError::GenericError(e)
    }
}

impl From<anyhow::Error> for RegistryDumpError {
    fn from(e: anyhow::Error) -> Self {
        RegistryDumpError::GenericError(e.to_string())
    }
}

impl From<RegistryDumpError> for anyhow::Error {
    fn from(e: RegistryDumpError) -> Self {
        match e {
            RegistryDumpError::RegistryClientError(e) => anyhow::anyhow!("Registry client error: {:?}", e),
            RegistryDumpError::LocalRegistryError(e) => anyhow::anyhow!("Registry error: {:?}", e),
            RegistryDumpError::IoError(e) => anyhow::anyhow!("IO error: {:?}", e),
            RegistryDumpError::SerdeError(e) => anyhow::anyhow!("Serde error: {:?}", e),
            RegistryDumpError::GenericError(e) => anyhow::anyhow!("Generic error: {:?}", e),
            RegistryDumpError::RecordNotFound => anyhow::anyhow!("Record not found"),
        }
    }
}
