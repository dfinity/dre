use std::path::PathBuf;
use std::{collections::BTreeMap, str::FromStr};

use crate::commands::subnet::Subnet;
use api_boundary_nodes::ApiBoundaryNodes;
use clap::Parser;
use clap::{error::ErrorKind, Args as ClapArgs};
use clap_num::maybe_hex;
use clio::*;
use completions::Completions;
use der_to_principal::DerToPrincipal;
use firewall::Firewall;
use get::Get;
use heal::Heal;
use hostos::HostOs;
use ic_management_types::{MinNakamotoCoefficients, Network, NodeFeature};
use neuron::Neuron;
use node_metrics::NodeMetrics;
use nodes::Nodes;
use proposals::Proposals;
use propose::Propose;
use qualify::Qualify;
use registry::Registry;
use update_authorized_subnets::UpdateAuthorizedSubnets;
use update_unassigned_nodes::UpdateUnassignedNodes;
use upgrade::Upgrade;
use url::Url;
use version::Version;
use vote::Vote;

use crate::auth::Neuron as AuthNeuron;

mod api_boundary_nodes;
mod completions;
mod der_to_principal;
mod firewall;
pub mod get;
mod heal;
pub mod hostos;
mod neuron;
mod node_metrics;
mod nodes;
mod proposals;
mod propose;
pub mod qualify;
mod registry;
mod subnet;
mod update_authorized_subnets;
mod update_unassigned_nodes;
pub mod upgrade;
mod version;
mod vote;

/// HSM authentication parameters
#[derive(ClapArgs, Debug, Clone)]
pub struct HsmParams {
    /// Slot that HSM key uses, can be read with pkcs11-tool
    #[clap(required = false,
        requires_all = ["hsm_slot","hsm_key_id", "hsm_pin"],
        conflicts_with = "private_key_pem",
        long, value_parser=maybe_hex::<u64>, global = true, env = "HSM_SLOT")]
    pub hsm_slot: u64,

    /// HSM Key ID, can be read with pkcs11-tool
    #[clap(
        required = false,
        requires_all = ["hsm_slot","hsm_key_id", "hsm_pin"],
        conflicts_with = "private_key_pem",
        long,
        global = true,
        env = "HSM_KEY_ID"
    )]
    pub hsm_key_id: String,
}

/// HSM authentication arguments
/// These comprise an optional PIN and optional parameters.
/// The PIN is used during autodetection if the optional
/// parameters are missing.
#[derive(ClapArgs, Debug, Clone)]
pub struct HsmOpts {
    /// Pin for the HSM key used for submitting proposals
    // Must be present if slot and key are specified.
    #[clap(
        required = false,
        alias = "hsm-pim",
        conflicts_with = "private_key_pem",
        long,
        global = true,
        hide_env_values = true,
        env = "HSM_PIN"
    )]
    pub hsm_pin: Option<String>,
    #[clap(flatten)]
    pub hsm_params: Option<HsmParams>,
}

// The following should ideally be defined in terms of an Enum
// as there is no conceivable scenario in which both a PEM file
// and a set of HSM options can be used by the program.
// Sadly, until ticket
//   https://github.com/clap-rs/clap/issues/2621
// is fixed, we cannot do this, and we must use a struct instead.
// Note that group(multiple = false) has no effect, and therefore
// we have to use conflicts and requires to specify option deps.
#[derive(ClapArgs, Debug, Clone)]
#[group(multiple = false)]
/// Authentication arguments
pub struct AuthOpts {
    /// Path to private key file (in PEM format)
    #[clap(
        long,
        required = false,
        global = true,
        conflicts_with_all = ["hsm_pin", "hsm_slot", "hsm_key_id"],
        env = "PRIVATE_KEY_PEM")]
    pub private_key_pem: Option<InputPath>,
    #[clap(flatten)]
    pub hsm_opts: HsmOpts,
}

impl TryFrom<PathBuf> for AuthOpts {
    type Error = clio::Error;
    fn try_from(path: PathBuf) -> Result<Self> {
        let p = Some(InputPath::new(&path)?);
        Ok(Self {
            private_key_pem: p,
            hsm_opts: HsmOpts {
                hsm_pin: None,
                hsm_params: None,
            },
        })
    }
}

impl TryFrom<String> for AuthOpts {
    type Error = clio::Error;
    fn try_from(path: String) -> Result<Self> {
        let p = Some(InputPath::new(&PathBuf::from(path))?);
        Ok(Self {
            private_key_pem: p,
            hsm_opts: HsmOpts {
                hsm_pin: None,
                hsm_params: None,
            },
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, author)]
pub struct Args {
    #[clap(flatten)]
    pub auth_opts: AuthOpts,

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

    /// Don't sync with the registry
    ///
    /// Useful for when the nns is unreachable
    #[clap(long)]
    pub no_sync: bool,

    /// Link to the related forum post, where proposal details can be discussed
    #[clap(long, global = true, visible_aliases = &["forum-link", "forum"])]
    pub forum_post_link: Option<String>,
}

macro_rules! impl_executable_command_for_enums {
    ($($var:ident),*) => {
        use crate::ctx::DreContext;
        use clap::{Subcommand, Command};

        #[derive(Subcommand, Debug)]
        pub enum Subcommands { $(
            $var($var),
        )*}

        impl ExecutableCommand for Subcommands {
            fn require_ic_admin(&self) -> IcAdminRequirement {
                match &self {
                    $(Subcommands::$var(variant) => variant.require_ic_admin(),)*
                }
            }

            async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
                match &self {
                    $(Subcommands::$var(variant) => variant.execute(ctx).await,)*
                }
            }

            fn validate(&self, cmd: &mut Command) {
                match &self {
                    $(Subcommands::$var(variant) => variant.validate(cmd),)*
                }
            }
        }
    }
}
pub(crate) use impl_executable_command_for_enums;

impl_executable_command_for_enums! { DerToPrincipal, Heal, Subnet, Get, Propose, UpdateUnassignedNodes, Version, NodeMetrics, HostOs, Nodes, ApiBoundaryNodes, Vote, Registry, Firewall, Upgrade, Proposals, Completions, Qualify, UpdateAuthorizedSubnets, Neuron }

pub trait ExecutableCommand {
    fn require_ic_admin(&self) -> IcAdminRequirement;

    fn validate(&self, cmd: &mut Command);

    fn execute(&self, ctx: DreContext) -> impl std::future::Future<Output = anyhow::Result<()>>;

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
    Anonymous,                                              // for get commands
    Detect,                                                 // detect the neuron
    OverridableBy { network: Network, neuron: AuthNeuron }, // eg automation which we know where is placed
}

impl ExecutableCommand for Args {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        self.subcommands.require_ic_admin()
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        self.subcommands.execute(ctx).await
    }

    fn validate(&self, cmd: &mut Command) {
        self.subcommands.validate(cmd)
    }
}
