use crate::commands::subnet::Subnet;
use crate::ctx::exe::impl_executable_command_for_enums;
use api_boundary_nodes::ApiBoundaryNodes;
use clap::Parser;
use completions::Completions;
use der_to_principal::DerToPrincipal;
use firewall::Firewall;
use get::Get;
use governance::Governance;
use hostos::HostOs;
use network::Network;
use neuron::Neuron;
use node_metrics::NodeMetrics;
use nodes::Nodes;
use proposals::Proposals;
use propose::Propose;
use qualify::Qualify;
use registry::Registry;
use strum::Display;
use update_authorized_subnets::UpdateAuthorizedSubnets;
use update_unassigned_nodes::UpdateUnassignedNodes;
use upgrade::Upgrade;
use url::Url;
use version::Version;
use vote::Vote;

pub(crate) mod api_boundary_nodes;
pub(crate) mod completions;
pub(crate) mod der_to_principal;
pub(crate) mod firewall;
pub mod get;
pub(crate) mod governance;
pub mod hostos;
pub(crate) mod network;
pub(crate) mod neuron;
pub(crate) mod node_metrics;
pub(crate) mod nodes;
pub(crate) mod proposals;
pub(crate) mod propose;
pub mod qualify;
pub(crate) mod registry;
pub(crate) mod subnet;
pub(crate) mod update_authorized_subnets;
pub(crate) mod update_unassigned_nodes;
pub mod upgrade;
pub(crate) mod version;
pub(crate) mod vote;
use crate::auth::AuthOpts;

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, author)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) auth_opts: AuthOpts,

    /// Neuron ID
    #[clap(long, global = true, env = "NEURON_ID", visible_aliases = &["neuron", "proposer"])]
    pub neuron_id: Option<u64>,

    /// Path to explicitly state ic-admin path to use
    #[clap(long, global = true, env = "IC_ADMIN")]
    pub ic_admin: Option<String>,

    #[clap(long, global = true, env = "IC_ADMIN_VERSION", default_value = "from-governance", value_parser = clap::value_parser!(IcAdminVersion), help = r#"Specify the version of ic admin to use
Options:
    1. from-governance, governance, govn, g => same as governance canister
    2. default, d => strict default version, embedded at build time
    3. <commit> => specific commit"#)]
    pub ic_admin_version: IcAdminVersion,

    #[clap(
        long,
        env = "NETWORK",
        default_value = "mainnet",
        help = r#"Specify the target network:
    - "mainnet",
    - "staging",
    - "<testnet>"
"#
    )]
    pub network: String,

    #[clap(long, env = "NNS_URLS", value_delimiter = ',', aliases = ["registry-url", "nns-url"], help = r#"NNS_URLs for target network, comma separated.
The argument is mandatory for testnets, and is optional for mainnet and staging"#)]
    pub nns_urls: Vec<Url>,

    #[clap(subcommand)]
    pub subcommands: Subcommands,

    /// To print as much information as possible
    #[clap(long, env = "VERBOSE", global = true)]
    pub verbose: bool,

    /// Run the tool offline when possible, i.e., do not sync registry and public dashboard data before the run
    ///
    /// Useful for when the NNS or Public dashboard are unreachable
    #[clap(long)]
    pub offline: bool,

    /// Path to file which contains cordoned features
    #[clap(long, global = true, visible_aliases = &["cf-file", "cfff"])]
    pub cordoned_features_file: Option<String>,
}

impl_executable_command_for_enums! { Args, DerToPrincipal, Network, Subnet, Get, Propose, UpdateUnassignedNodes, Version, NodeMetrics, HostOs, Nodes, ApiBoundaryNodes, Vote, Registry, Firewall, Upgrade, Proposals, Completions, Qualify, UpdateAuthorizedSubnets, Neuron, Governance }

#[derive(Debug, Display, Clone)]
pub enum IcAdminVersion {
    FromGovernance,
    Fallback,
    Strict(String),
}

impl From<&str> for IcAdminVersion {
    fn from(value: &str) -> Self {
        match value {
            "from-governance" | "governance" | "g" | "govn" => Self::FromGovernance,
            "fallback" | "f" => Self::Fallback,
            s => Self::Strict(s.to_string()),
        }
    }
}
