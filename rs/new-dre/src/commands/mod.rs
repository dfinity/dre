use std::{path::PathBuf, str::FromStr};

use api_boundary_nodes::ApiBoundaryNodes;
use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use der_to_principal::DerToPrincipal;
use firewall::Firewall;
use get::Get;
use heal::Heal;
use hostos::HostOsCmd;
use ic_canisters::governance::governance_canister_version;
use ic_management_types::Network;
use nodes::Nodes;
use proposals::Proposals;
use propose::Propose;
use registry::Registry;
use trustworthy_metrics::TrustworthyMetrics;
use update_unassigned_nodes::UpdateUnassignedNodes;
use upgrade::Upgrade;
use url::Url;
use version::VersionCmd;
use vote::Vote;

use crate::ctx::DreContext;

mod api_boundary_nodes;
mod der_to_principal;
mod firewall;
mod get;
mod heal;
pub mod hostos;
mod nodes;
mod proposals;
mod propose;
mod registry;
mod subnet;
mod trustworthy_metrics;
mod update_unassigned_nodes;
pub mod upgrade;
mod version;
mod vote;

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, author)]
pub struct Args {
    /// Pin for the HSM key used for submitting proposals
    #[clap(long, global = true, hide_env_values = true, env = "HSM_PIN")]
    pub hsm_pin: Option<String>,

    /// Slot that HSM key uses, can be read with pkcs11-tool
    #[clap(long, value_parser=maybe_hex::<u64>, global = true, env = "HSM_SLOT")]
    pub hsm_slot: Option<u64>,

    /// HSM Key ID, can be read with pkcs11-tool
    #[clap(long, global = true, env = "HSM_KEY_ID")]
    pub hsm_key_id: Option<String>,

    /// Path to key pem file
    #[clap(long, global = true, env = "PRIVATE_KEY_PEM")]
    pub private_key_pem: Option<PathBuf>,

    /// Neuron ID
    #[clap(long, global = true, env = "NEURON_ID")]
    pub neuron_id: Option<u64>,

    /// Path to explicitly state ic-admin path to use
    #[clap(long, global = true, env = "IC_ADMIN")]
    pub ic_admin: Option<String>,

    /// To skip the confirmation prompt
    #[clap(short, long, global = true, env = "YES", conflicts_with = "dry_run")]
    pub yes: bool,

    #[clap(long, aliases = [ "dry-run", "dryrun", "simulate", "no"], global = true, conflicts_with = "yes", help = r#"Dry-run, or simulate proposal submission. If specified will not submit the proposal
but will show the ic-admin command and the proposal payload"#)]
    pub dry_run: bool,

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
}

#[derive(Subcommand, Debug)]
pub enum Subcommands {
    /// Convert a DER file to a Principal
    DerToPrincipal(DerToPrincipal),

    /// Heal subnets
    Heal(Heal),

    /// Manage subnets
    Subnet(subnet::SubnetCommand),

    /// Get a value using ic-admin CLI
    Get(Get),

    /// Place a proposal using the ic-admin CLI
    Propose(Propose),

    /// Place a proposal for updating unassigned nodes
    UpdateUnassignedNodes(UpdateUnassignedNodes),

    /// Manage versions
    Version(VersionCmd),

    /// Manage hostos versions
    HostOs(HostOsCmd),

    /// Manage nodes
    Nodes(Nodes),

    /// Manage api boundary nodes
    ApiBoundaryNodes(ApiBoundaryNodes),

    /// Vote on our proposals
    Vote(Vote),

    /// Trustworthy Metrics
    TrustworthyMetrics(TrustworthyMetrics),

    /// Registry inspection (dump) operations
    Registry(Registry),

    /// Firewall rules
    Firewall(Firewall),

    /// Upgrade
    Upgrade(Upgrade),

    /// Proposals
    Proposals(Proposals),
}

pub trait ExecutableCommand {
    fn require_neuron(&self) -> bool;

    fn require_registry(&self) -> bool;

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()>;
}

impl ExecutableCommand for Args {
    fn require_neuron(&self) -> bool {
        match &self.subcommands {
            Subcommands::DerToPrincipal(c) => c.require_neuron(),
            Subcommands::Heal(c) => c.require_neuron(),
            Subcommands::Subnet(c) => c.require_neuron(),
            Subcommands::Get(c) => c.require_neuron(),
            Subcommands::Propose(c) => c.require_neuron(),
            Subcommands::UpdateUnassignedNodes(c) => c.require_neuron(),
            Subcommands::Version(c) => c.require_neuron(),
            Subcommands::HostOs(c) => c.require_neuron(),
            Subcommands::Nodes(c) => c.require_neuron(),
            Subcommands::ApiBoundaryNodes(c) => c.require_neuron(),
            Subcommands::Vote(c) => c.require_neuron(),
            Subcommands::TrustworthyMetrics(c) => c.require_neuron(),
            Subcommands::Registry(c) => c.require_neuron(),
            Subcommands::Firewall(c) => c.require_neuron(),
            Subcommands::Upgrade(c) => c.require_neuron(),
            Subcommands::Proposals(c) => c.require_neuron(),
        }
    }

    fn require_registry(&self) -> bool {
        match &self.subcommands {
            Subcommands::DerToPrincipal(c) => c.require_registry(),
            Subcommands::Heal(c) => c.require_registry(),
            Subcommands::Subnet(c) => c.require_registry(),
            Subcommands::Get(c) => c.require_registry(),
            Subcommands::Propose(c) => c.require_registry(),
            Subcommands::UpdateUnassignedNodes(c) => c.require_registry(),
            Subcommands::Version(c) => c.require_registry(),
            Subcommands::HostOs(c) => c.require_registry(),
            Subcommands::Nodes(c) => c.require_registry(),
            Subcommands::ApiBoundaryNodes(c) => c.require_registry(),
            Subcommands::Vote(c) => c.require_registry(),
            Subcommands::TrustworthyMetrics(c) => c.require_registry(),
            Subcommands::Registry(c) => c.require_registry(),
            Subcommands::Firewall(c) => c.require_registry(),
            Subcommands::Upgrade(c) => c.require_registry(),
            Subcommands::Proposals(c) => c.require_registry(),
        }
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        match &self.subcommands {
            Subcommands::DerToPrincipal(c) => c.execute(ctx).await,
            Subcommands::Heal(c) => c.execute(ctx).await,
            Subcommands::Subnet(c) => c.execute(ctx).await,
            Subcommands::Get(c) => c.execute(ctx).await,
            Subcommands::Propose(c) => c.execute(ctx).await,
            Subcommands::UpdateUnassignedNodes(c) => c.execute(ctx).await,
            Subcommands::Version(c) => c.execute(ctx).await,
            Subcommands::HostOs(c) => c.execute(ctx).await,
            Subcommands::Nodes(c) => c.execute(ctx).await,
            Subcommands::ApiBoundaryNodes(c) => c.execute(ctx).await,
            Subcommands::Vote(c) => c.execute(ctx).await,
            Subcommands::TrustworthyMetrics(c) => c.execute(ctx).await,
            Subcommands::Registry(c) => c.execute(ctx).await,
            Subcommands::Firewall(c) => c.execute(ctx).await,
            Subcommands::Upgrade(c) => c.execute(ctx).await,
            Subcommands::Proposals(c) => c.execute(ctx).await,
        }
    }
}
