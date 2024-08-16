use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    rc::Rc,
    str::FromStr,
};

use clap::Args;
use ic_management_backend::{
    health::{HealthClient, HealthStatusQuerier},
    lazy_registry::{LazyRegistry, LazyRegistryFamilyEntries},
    public_dashboard::query_ic_dashboard_list,
};
use ic_management_types::{Network, NodeProvidersResponse, Status};
use ic_protobuf::registry::{
    api_boundary_node::v1::ApiBoundaryNodeRecord,
    dc::v1::DataCenterRecord,
    hostos_version::v1::HostosVersionRecord,
    node::v1::{ConnectionEndpoint, IPv4InterfaceConfig, NodeRecord},
    node_operator::v1::NodeOperatorRecord,
    node_rewards::v2::NodeRewardsTable,
    replica_version::v1::ReplicaVersionRecord,
    subnet::v1::{ChainKeyConfig, EcdsaConfig, SubnetFeatures, SubnetRecord as SubnetRecordProto},
    unassigned_nodes_config::v1::UnassignedNodesConfigRecord,
};
use ic_registry_subnet_type::SubnetType;
use ic_types::PrincipalId;
use itertools::Itertools;
use log::{info, warn};
use serde::Serialize;

use crate::ctx::DreContext;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Registry {
    /// Output file (default is stdout)
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Output only information related to the node operator records with incorrect rewards
    #[clap(long)]
    pub incorrect_rewards: bool,
}

impl ExecutableCommand for Registry {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        let writer: Box<dyn std::io::Write> = match &self.output {
            Some(path) => {
                let path = path.with_extension("json").canonicalize()?;
                info!("Writing to file: {:?}", path);
                Box::new(std::io::BufWriter::new(fs_err::File::create(path)?))
            }
            None => Box::new(std::io::stdout()),
        };

        if self.incorrect_rewards {
            let node_operators = self.get_registry(ctx).await?;
            let node_operators = node_operators.node_operators.iter().filter(|rec| !rec.rewards_correct).collect_vec();
            serde_json::to_writer_pretty(writer, &node_operators)?;
        } else {
            serde_json::to_writer_pretty(writer, &self.get_registry(ctx).await?)?;
        }

        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}

impl Registry {
    async fn get_registry(&self, ctx: DreContext) -> anyhow::Result<RegistryDump> {
        let local_registry = ctx.registry().await;

        let elected_guest_os_versions = get_elected_guest_os_versions(&local_registry)?;
        let elected_host_os_versions = get_elected_host_os_versions(&local_registry)?;

        let node_provider_names: HashMap<PrincipalId, String> = HashMap::from_iter(
            query_ic_dashboard_list::<NodeProvidersResponse>(ctx.network(), "v3/node-providers")
                .await?
                .node_providers
                .iter()
                .map(|np| (np.principal_id, np.display_name.clone())),
        );
        let mut node_operators = get_node_operators(&local_registry, &node_provider_names, ctx.network())?;

        let dcs = local_registry
            .get_family_entries_versioned::<DataCenterRecord>()
            .map_err(|e| anyhow::anyhow!("Couldn't get data centers: {:?}", e))?
            .into_iter()
            .map(|(_, (_, record))| record)
            .collect();

        let subnets = get_subnets(&local_registry)?;

        let unassigned_nodes_config = get_unassigned_nodes(&local_registry)?;

        let nodes = get_nodes(&local_registry, &node_operators, &subnets, ctx.network()).await?;

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
        })
    }
}

fn get_elected_guest_os_versions(local_registry: &Rc<LazyRegistry>) -> anyhow::Result<Vec<ReplicaVersionRecord>> {
    let elected_versions = local_registry
        .get_family_entries_versioned::<ReplicaVersionRecord>()
        .map_err(|e| anyhow::anyhow!("Couldn't get elected versions: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .collect();
    Ok(elected_versions)
}

fn get_elected_host_os_versions(local_registry: &Rc<LazyRegistry>) -> anyhow::Result<Vec<HostosVersionRecord>> {
    let elected_versions = local_registry
        .get_family_entries_versioned::<HostosVersionRecord>()
        .map_err(|e| anyhow::anyhow!("Couldn't get elected versions: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .collect();
    Ok(elected_versions)
}

fn get_node_operators(
    local_registry: &Rc<LazyRegistry>,
    node_provider_names: &HashMap<PrincipalId, String>,
    network: &Network,
) -> anyhow::Result<BTreeMap<PrincipalId, NodeOperator>> {
    let all_nodes = local_registry
        .get_family_entries_versioned::<NodeRecord>()
        .map_err(|e| anyhow::anyhow!("Couldn't get nodes: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| (k, record))
        .collect::<BTreeMap<_, _>>();
    let node_operators = local_registry
        .get_family_entries_versioned::<NodeOperatorRecord>()
        .map_err(|e| anyhow::anyhow!("Couldn't get node operators: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| {
            let node_operator_principal_id = PrincipalId::from_str(k.as_str()).expect("Couldn't parse principal id");
            let node_provider_name = node_provider_names
                .get(&PrincipalId::try_from(&record.node_provider_principal_id).expect("Couldn't parse principal id"))
                .map_or_else(String::default, |v| {
                    if network.is_mainnet() && v.is_empty() {
                        panic!("Node provider name should not be empty for mainnet")
                    }
                    v.to_string()
                });
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

fn get_subnets(local_registry: &Rc<LazyRegistry>) -> anyhow::Result<Vec<SubnetRecord>> {
    Ok(local_registry
        .get_family_entries::<SubnetRecordProto>()?
        .into_iter()
        .map(|(subnet_id, record)| SubnetRecord {
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
            start_as_nns: record.start_as_nns,
            subnet_type: SubnetType::try_from(record.subnet_type).unwrap(),
            features: record.features.clone().unwrap_or_default(),
            max_number_of_canisters: record.max_number_of_canisters,
            ssh_readonly_access: record.ssh_readonly_access,
            ssh_backup_access: record.ssh_backup_access,
            ecdsa_config: record.ecdsa_config,
            dkg_dealings_per_block: record.dkg_dealings_per_block,
            is_halted: record.is_halted,
            halt_at_cup_height: record.halt_at_cup_height,
            chain_key_config: record.chain_key_config,
        })
        .collect::<Vec<_>>())
}

fn get_unassigned_nodes(local_registry: &Rc<LazyRegistry>) -> anyhow::Result<Option<UnassignedNodesConfigRecord>> {
    let unassigned_nodes_config = local_registry
        .get_family_entries_versioned::<UnassignedNodesConfigRecord>()
        .map_err(|e| anyhow::anyhow!("Couldn't get unassigned nodes config: {:?}", e))?
        .into_iter()
        .map(|(_, (_, record))| record)
        .next();
    Ok(unassigned_nodes_config)
}

async fn get_nodes(
    local_registry: &Rc<LazyRegistry>,
    node_operators: &BTreeMap<PrincipalId, NodeOperator>,
    subnets: &[SubnetRecord],
    network: &Network,
) -> anyhow::Result<Vec<NodeDetails>> {
    let health_client = HealthClient::new(network.clone());
    let nodes_health = health_client.nodes().await?;

    let nodes = local_registry
        .get_family_entries_versioned::<NodeRecord>()
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
                node_provider_id: match node_operators.get(&node_operator_id) {
                    Some(no) => no.node_provider_principal_id,
                    None => PrincipalId::new_anonymous(),
                },
                subnet_id: subnets
                    .iter()
                    .find(|subnet| subnet.membership.contains(&k))
                    .map(|subnet| subnet.subnet_id),
                dc_id: match node_operators.get(&node_operator_id) {
                    Some(no) => no.dc_id.clone(),
                    None => "".to_string(),
                },
                status: nodes_health.get(&node_id).unwrap_or(&ic_management_types::Status::Unknown).clone(),
            }
        })
        .collect::<Vec<_>>();
    Ok(nodes)
}

fn get_node_rewards_table(local_registry: &Rc<LazyRegistry>, network: &Network) -> NodeRewardsTableFlattened {
    let rewards_table_bytes = local_registry.get_family_entries::<NodeRewardsTable>();

    let mut rewards_table = match rewards_table_bytes {
        Ok(r) => r,
        Err(_) => {
            if network.is_mainnet() {
                panic!("Failed to get Node Rewards Table for mainnet")
            } else {
                warn!("Failed to get Node Rewards Table for {}", network.name);
                BTreeMap::new()
            }
        }
    };

    let table = match rewards_table.first_entry() {
        Some(f) => f.get().table.clone(),
        None => {
            if network.is_mainnet() {
                panic!("Failed to get Node Rewards Table for mainnet")
            } else {
                warn!("Failed to get Node Rewards Table for {}", network.name);
                BTreeMap::new()
            }
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

fn get_api_boundary_nodes(local_registry: &Rc<LazyRegistry>) -> anyhow::Result<Vec<ApiBoundaryNodeDetails>> {
    let api_bns = local_registry
        .get_family_entries_versioned::<ApiBoundaryNodeRecord>()
        .map_err(|e| anyhow::anyhow!("Couldn't get api boundary nodes: {:?}", e))?
        .into_iter()
        .map(|(k, (_, record))| {
            let principal = PrincipalId::from_str(k.as_str()).expect("Couldn't parse principal id");
            ApiBoundaryNodeDetails {
                principal,
                version: record.version,
            }
        })
        .collect();

    Ok(api_bns)
}

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
    dkg_dealings_per_block: u64,
    start_as_nns: bool,
    subnet_type: SubnetType,
    features: SubnetFeatures,
    max_number_of_canisters: u64,
    ssh_readonly_access: Vec<String>,
    ssh_backup_access: Vec<String>,
    ecdsa_config: Option<EcdsaConfig>,
    is_halted: bool,
    halt_at_cup_height: bool,
    chain_key_config: Option<ChainKeyConfig>,
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
