use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use der_to_principal::DerToPrincipal;
use get::Get;
use heal::Heal;
use hostos::HostOsCmd;
use nodes::Nodes;
use propose::Propose;
use update_unassigned_nodes::UpdateUnassignedNodes;
use url::Url;
use version::VersionCmd;

mod der_to_principal;
mod get;
mod heal;
mod hostos;
mod nodes;
mod propose;
mod subnet;
mod update_unassigned_nodes;
mod version;

#[derive(Parser, Debug)]
#[clap(version, about, author)]
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
    pub private_key_pem: Option<String>,

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
}
