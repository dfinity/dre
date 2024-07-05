use std::{cell::RefCell, path::PathBuf, rc::Rc, str::FromStr};

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

mod api_boundary_nodes;
mod der_to_principal;
mod firewall;
mod get;
mod heal;
mod hostos;
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

    /// Bellow options are used for caching

    #[clap(skip)]
    network_cache: Rc<RefCell<Option<Network>>>,

    #[clap(skip)]
    private_key_pem_cache: Rc<RefCell<Option<PathBuf>>>,

    #[clap(skip)]
    neuron_cache: Rc<RefCell<Option<u64>>>,

    #[clap(skip)]
    governance_canister_version_hash_cache: Rc<RefCell<Option<String>>>,
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

const STAGING_NEURON_ID: u64 = 49;
impl Args {
    pub async fn get_network(&self) -> anyhow::Result<Network> {
        if let Some(ref network) = *self.network_cache.borrow() {
            return Ok(network.clone());
        }

        let target_network = ic_management_types::Network::new(self.network.clone(), &self.nns_urls)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        if target_network.name == "staging" {
            if self.private_key_pem.is_none() {
                let path = PathBuf::from_str(&std::env::var("HOME")?)?.join("/.config/dfx/identity/bootstrap-super-leader/identity.pem");
                if path.exists() {
                    *self.private_key_pem_cache.borrow_mut() = Some(path)
                }
            }
            if self.neuron_id.is_none() {
                *self.neuron_cache.borrow_mut() = Some(STAGING_NEURON_ID);
            }
        }

        *self.network_cache.borrow_mut() = Some(target_network.clone());
        Ok(target_network)
    }

    pub async fn get_governance_canister_version_hash(&self) -> anyhow::Result<String> {
        if let Some(ref hash) = *self.governance_canister_version_hash_cache.borrow() {
            return Ok(hash.clone());
        }

        let target_network = self.get_network().await?;
        let governance_canister_version = governance_canister_version(target_network.get_nns_urls()).await?;

        *self.governance_canister_version_hash_cache.borrow_mut() = Some(governance_canister_version.stringified_hash.clone());
        Ok(governance_canister_version.stringified_hash)
    }
}
