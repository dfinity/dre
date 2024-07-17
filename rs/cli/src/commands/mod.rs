use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

use api_boundary_nodes::ApiBoundaryNodes;
use clap::{error::ErrorKind, Command, Parser, Subcommand};
use clap_num::maybe_hex;
use completions::Completions;
use der_to_principal::DerToPrincipal;
use firewall::Firewall;
use get::Get;
use heal::Heal;
use hostos::HostOsCmd;
use ic_management_types::{MinNakamotoCoefficients, Network, NodeFeature};
use nodes::Nodes;
use proposals::Proposals;
use propose::Propose;
use qualify::QualifyCmd;
use registry::Registry;
use trustworthy_metrics::TrustworthyMetrics;
use update_unassigned_nodes::UpdateUnassignedNodes;
use upgrade::Upgrade;
use url::Url;
use version::VersionCmd;
use vote::Vote;

use crate::{auth::Neuron, ctx::DreContext};

mod api_boundary_nodes;
mod completions;
mod der_to_principal;
mod firewall;
mod get;
mod heal;
pub mod hostos;
mod nodes;
mod proposals;
mod propose;
mod qualify;
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

    /// To print as much information as possible
    #[clap(long, env = "VERBOSE", global = true)]
    pub verbose: bool,
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

    /// Completions
    Completions(Completions),

    /// Qualification
    Qualify(QualifyCmd),
}

pub trait ExecutableCommand {
    fn require_ic_admin(&self) -> IcAdminRequirement;

    fn validate(&self, cmd: &mut Command);

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()>;

    fn validate_min_nakamoto_coefficients(cmd: &mut clap::Command, min_nakamoto_coefficients: &[String]) {
        let _ = Self::_parse_min_nakamoto_coefficients_inner(Some(cmd), min_nakamoto_coefficients);
    }

    fn parse_min_nakamoto_coefficients(min_nakamoto_coefficients: &[String]) -> Option<MinNakamotoCoefficients> {
        Self::_parse_min_nakamoto_coefficients_inner(None, min_nakamoto_coefficients)
    }

    fn _parse_min_nakamoto_coefficients_inner(
        cmd: Option<&mut clap::Command>,
        min_nakamoto_coefficients: &[String],
    ) -> Option<MinNakamotoCoefficients> {
        let min_nakamoto_coefficients: Vec<String> = if min_nakamoto_coefficients.is_empty() {
            ["node_provider=5", "average=3"].iter().map(|s| String::from(*s)).collect()
        } else {
            min_nakamoto_coefficients.to_vec()
        };

        let mut average = 3.0;
        let mut coefficients = BTreeMap::new();
        for entry in min_nakamoto_coefficients {
            let (key, value) = match entry.split_once('=') {
                Some(s) => s,
                None => {
                    if let Some(cmd) = cmd {
                        cmd.error(ErrorKind::ValueValidation, "Falied to parse feature from string").exit()
                    }
                    continue;
                }
            };

            if key.to_lowercase() == "average" {
                average = match value.parse::<f64>() {
                    Ok(a) => a,
                    Err(_) => {
                        if let Some(cmd) = cmd {
                            cmd.error(ErrorKind::ValueValidation, "Falied to parse feature from string").exit()
                        }
                        continue;
                    }
                };
                continue;
            } else {
                let feature = match NodeFeature::from_str(key) {
                    Ok(v) => v,
                    Err(_) => {
                        if let Some(cmd) = cmd {
                            cmd.error(ErrorKind::ValueValidation, "Falied to parse feature from string").exit()
                        }
                        continue;
                    }
                };
                let val = match value.parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        if let Some(cmd) = cmd {
                            cmd.error(ErrorKind::ValueValidation, "Falied to parse feature from string").exit()
                        }
                        continue;
                    }
                };
                coefficients.insert(feature, val);
            }
        }

        Some(MinNakamotoCoefficients { coefficients, average })
    }
}

pub enum IcAdminRequirement {
    None,
    Anonymous,                                          // for get commands
    Detect,                                             // detect the neuron
    OverridableBy { network: Network, neuron: Neuron }, // eg automation which we know where is placed
}

impl ExecutableCommand for Args {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommands {
            Subcommands::DerToPrincipal(c) => c.require_ic_admin(),
            Subcommands::Heal(c) => c.require_ic_admin(),
            Subcommands::Subnet(c) => c.require_ic_admin(),
            Subcommands::Get(c) => c.require_ic_admin(),
            Subcommands::Propose(c) => c.require_ic_admin(),
            Subcommands::UpdateUnassignedNodes(c) => c.require_ic_admin(),
            Subcommands::Version(c) => c.require_ic_admin(),
            Subcommands::HostOs(c) => c.require_ic_admin(),
            Subcommands::Nodes(c) => c.require_ic_admin(),
            Subcommands::ApiBoundaryNodes(c) => c.require_ic_admin(),
            Subcommands::Vote(c) => c.require_ic_admin(),
            Subcommands::TrustworthyMetrics(c) => c.require_ic_admin(),
            Subcommands::Registry(c) => c.require_ic_admin(),
            Subcommands::Firewall(c) => c.require_ic_admin(),
            Subcommands::Upgrade(c) => c.require_ic_admin(),
            Subcommands::Proposals(c) => c.require_ic_admin(),
            Subcommands::Completions(c) => c.require_ic_admin(),
            Subcommands::Qualify(c) => c.require_ic_admin(),
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
            Subcommands::Completions(c) => c.execute(ctx).await,
            Subcommands::Qualify(c) => c.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut Command) {
        match &self.subcommands {
            Subcommands::DerToPrincipal(c) => c.validate(cmd),
            Subcommands::Heal(c) => c.validate(cmd),
            Subcommands::Subnet(c) => c.validate(cmd),
            Subcommands::Get(c) => c.validate(cmd),
            Subcommands::Propose(c) => c.validate(cmd),
            Subcommands::UpdateUnassignedNodes(c) => c.validate(cmd),
            Subcommands::Version(c) => c.validate(cmd),
            Subcommands::HostOs(c) => c.validate(cmd),
            Subcommands::Nodes(c) => c.validate(cmd),
            Subcommands::ApiBoundaryNodes(c) => c.validate(cmd),
            Subcommands::Vote(c) => c.validate(cmd),
            Subcommands::TrustworthyMetrics(c) => c.validate(cmd),
            Subcommands::Registry(c) => c.validate(cmd),
            Subcommands::Firewall(c) => c.validate(cmd),
            Subcommands::Upgrade(c) => c.validate(cmd),
            Subcommands::Proposals(c) => c.validate(cmd),
            Subcommands::Completions(c) => c.validate(cmd),
            Subcommands::Qualify(c) => c.validate(cmd),
        }
    }
}
