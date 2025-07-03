use crate::ctx::DreContext;
use crate::{auth::AuthRequirement, exe::args::GlobalArgs, exe::ExecutableCommand};
use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_canisters::IcAgentCanisterClient;
use ic_management_backend::{health::HealthStatusQuerier, lazy_registry::LazyRegistry};
use ic_management_types::{HealthStatus, Network, Operator};
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
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::cmp::max;
use std::iter::{IntoIterator, Iterator};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::Ipv6Addr,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

#[derive(Args, Debug)]
#[clap(after_help = r#"EXAMPLES:
    dre registry                                                         # Dump all contents to stdout
    dre registry --filter rewards_correct!=true              # Entries for which rewardable_nodes != total_up_nodes
    dre registry --filter "node_type=type1"                              # Entries where node_type == "type1"
    dre registry -o registry.json --filter "subnet_id startswith tdb26"  # Write to file and filter by subnet_id
    dre registry -o registry.json --filter "node_id contains h5zep"      # Write to file and filter by node_id"#)]
pub struct Registry {
    /// Output file (default is stdout)
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Filters in `key=value` format
    #[clap(long, short, alias = "filter")]
    pub filters: Vec<Filter>,

    /// Specify the height for the registry
    #[clap(long, visible_aliases = ["registry-height", "version"])]
    pub height: Option<u64>,
}

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
            anyhow::bail!("Expected format: `key comparison value` (spaces around the comparison are optional, supported comparison: = != > < >= <= re contains startswith endswith), found {}", s);
        }
    }
}

impl ExecutableCommand for Registry {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        let writer: Box<dyn std::io::Write> = match &self.output {
            Some(path) => {
                let path = path.with_extension("json");
                let file = fs_err::File::create(path)?;
                info!("Writing to file: {:?}", file.path().canonicalize()?);
                Box::new(std::io::BufWriter::new(file))
            }
            None => Box::new(std::io::stdout()),
        };

        let registry = self.get_registry(ctx).await?;

        let mut serde_value = serde_json::to_value(registry)?;
        self.filters.iter().for_each(|filter| {
            filter_json_value(&mut serde_value, &filter.key, &filter.value, &filter.comparison);
        });

        serde_json::to_writer_pretty(writer, &serde_value)?;

        Ok(())
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}

impl Registry {
    async fn get_registry(&self, ctx: DreContext) -> anyhow::Result<RegistryDump> {
        let local_registry = ctx.registry_with_version(self.height).await;

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
            node_providers: get_node_providers(&local_registry, ctx.network(), ctx.is_offline(), self.height.is_none()).await?,
        })
    }
}

fn get_elected_guest_os_versions(local_registry: &Arc<dyn LazyRegistry>) -> anyhow::Result<Vec<ReplicaVersionRecord>> {
    local_registry.elected_guestos_records()
}

fn get_elected_host_os_versions(local_registry: &Arc<dyn LazyRegistry>) -> anyhow::Result<Vec<HostosVersionRecord>> {
    local_registry.elected_hostos_records()
}

fn fetch_max_rewardable_count(record: &Operator, nodes_in_registry: u64) -> BTreeMap<String, u32> {
    let mut exceptions: HashMap<PrincipalId, BTreeMap<String, u32>> = [
        (
            "ukji3-ju5bx-ty5r7-qwk4p-eobil-isp26-fsomg-44kwf-j4ew7-ozkqy-wqe",
            BTreeMap::from([("type3".to_string(), 16), ("type3.1".to_string(), 9)]),
        ),
        (
            "sm6rh-sldoa-opp4o-d7ckn-y4r2g-eoqhv-nymok-teuto-4e5ep-yt6ky-bqe",
            BTreeMap::from([("type1.1".to_string(), 1)]),
        ),
        (
            "xph6u-z3z2t-s7hh7-gtlxh-bbgbx-aatlm-eab4o-bsank-nqruh-3ub4q-sae",
            BTreeMap::from([("type1.1".to_string(), 28)]),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (PrincipalId::from_str(k).unwrap(), v))
    .collect::<HashMap<_, _>>();

    if let Some(exception_max_rewardables) = exceptions.remove(&record.principal) {
        return exception_max_rewardables;
    }
    let mut non_zeros_rewardable_nodes = record
        .rewardable_nodes
        .clone()
        .into_iter()
        .filter(|(_, v)| *v > 0)
        .collect::<BTreeMap<_, _>>();
    let nodes_rewards_types_count = non_zeros_rewardable_nodes.values().map(|count| *count as u64).sum::<u64>();
    let node_allowance_total = record.node_allowance + nodes_in_registry;

    if node_allowance_total == nodes_rewards_types_count {
        return non_zeros_rewardable_nodes;
    }
    if non_zeros_rewardable_nodes.len() == 1 {
        let (node_rewards_type, node_rewards_type_count) = non_zeros_rewardable_nodes.pop_first().unwrap();
        let max_rewardable_count = max(node_rewards_type_count, nodes_in_registry as u32);
        return BTreeMap::from([(node_rewards_type, max_rewardable_count)]);
    }

    if nodes_in_registry == 0 {
        return BTreeMap::new();
    }
    warn!(
        "Node operator {} has {} nodes in registry with {} node allowance total, but {} reward types!  The rewardable nodes by reward type for this operator are: {:?}",
        record.principal, nodes_in_registry, node_allowance_total, nodes_rewards_types_count, record.rewardable_nodes
    );
    BTreeMap::new()
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
            let max_rewardable_count = fetch_max_rewardable_count(record, nodes_in_registry);
            let node_allowance_total = record.node_allowance + nodes_in_registry;

            (
                record.principal,
                NodeOperator {
                    node_operator_principal_id: *k,
                    node_provider_principal_id: record.provider.principal,
                    dc_id: record.datacenter.as_ref().map(|d| d.name.to_owned()).unwrap_or_default(),
                    rewardable_nodes: record.rewardable_nodes.clone(),
                    node_allowance: record.node_allowance,
                    ipv6: Some(record.ipv6.to_string()),
                    computed: NodeOperatorComputed {
                        node_provider_name,
                        node_allowance_remaining: record.node_allowance,
                        node_allowance_total,
                        total_up_nodes: 0,
                        nodes_health: Default::default(),
                        max_rewardable_count,
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

    // Rewardable nodes for all node operators
    let mut rewardable_nodes: IndexMap<PrincipalId, BTreeMap<String, u32>> = node_operators
        .iter()
        .map(|(k, v)| (*k, v.computed.max_rewardable_count.clone()))
        .collect();

    let nodes = local_registry.nodes().await.map_err(|e| anyhow::anyhow!("Couldn't get nodes: {:?}", e))?;

    for (_, record) in nodes.iter() {
        let node_operator_id = record.operator.principal;
        let rewardable_nodes = rewardable_nodes.get_mut(&node_operator_id);

        if let Some(rewardable_nodes) = rewardable_nodes {
            let table_node_reward_type = match record.node_reward_type.unwrap_or(NodeRewardType::Unspecified) {
                NodeRewardType::Type0 => "type0",
                NodeRewardType::Type1 => "type1",
                NodeRewardType::Type2 => "type2",
                NodeRewardType::Type3 => "type3",
                NodeRewardType::Type1dot1 => "type1.1",
                NodeRewardType::Type3dot1 => "type3.1",
                NodeRewardType::Unspecified => "unspecified",
            };

            rewardable_nodes
                .entry(table_node_reward_type.to_string())
                .and_modify(|count| *count = 0.max(count.saturating_sub(1)));

        }
    }

    let nodes = nodes
        .iter()
        .map(|(k, record)| {
            let node_operator_id = record.operator.principal;
            let node_reward_type = record.node_reward_type.unwrap_or(NodeRewardType::Unspecified);
            let rewardable_nodes = rewardable_nodes.get_mut(&node_operator_id);

            let node_reward_type = if node_reward_type != NodeRewardType::Unspecified {
                node_reward_type.as_str_name().to_string()
            } else {
                match rewardable_nodes {
                    Some(rewardable_nodes) => match rewardable_nodes.len() {
                        0 => "unknown:no_rewardable_nodes_found".to_string(),
                        1 => {
                            let (k, mut v) = rewardable_nodes.pop_first().unwrap_or_else(|| ("unknown".to_string(), 0));
                            v = v.saturating_sub(1);
                            if v != 0 {
                                rewardable_nodes.insert(k.clone(), v);
                            }
                            format!("estimated:{}", k)
                        }
                        _ => "unknown:multiple_rewardable_nodes".to_string(),
                    },
                    None => "unknown".to_string(),
                }
            };

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
                subnet_id: subnets
                    .iter()
                    .find(|subnet| subnet.membership.contains(&k.to_string()))
                    .map(|subnet| subnet.subnet_id),
                dc_id: match node_operators.get(&node_operator_id) {
                    Some(no) => no.dc_id.clone(),
                    None => "".to_string(),
                },
                status: nodes_health.get(k).unwrap_or(&ic_management_types::HealthStatus::Unknown).clone(),
                node_reward_type,
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
                    .counts_by(|dc_name| dc_name),
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
    node_allowance: u64,
    ipv6: Option<String>,
    computed: NodeOperatorComputed,
}

#[derive(Clone, Debug, Serialize)]
struct NodeOperatorComputed {
    node_provider_name: String,
    node_allowance_remaining: u64,
    node_allowance_total: u64,
    total_up_nodes: u32,
    nodes_health: IndexMap<String, Vec<PrincipalId>>,
    max_rewardable_count: BTreeMap<String, u32>,
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
    nodes_per_dc: HashMap<String, usize>,
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
                    if let Value::String(other) = other {
                        s.contains(other)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Comparison::StartsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        s.starts_with(other)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Comparison::EndsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        s.ends_with(other)
                    } else {
                        false
                    }
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
