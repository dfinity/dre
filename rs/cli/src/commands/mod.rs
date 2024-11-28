use std::path::PathBuf;

use crate::commands::subnet::Subnet;
use api_boundary_nodes::ApiBoundaryNodes;
use clap::Args as ClapArgs;
use clap::Parser;
use clap_num::maybe_hex;
use clio::*;
use completions::Completions;
use der_to_principal::DerToPrincipal;
use firewall::Firewall;
use get::Get;
use heal::Heal;
use hostos::HostOs;
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
pub(crate) mod heal;
pub mod hostos;
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

/// HSM authentication parameters
#[derive(ClapArgs, Debug, Clone)]
pub(crate) struct HsmParams {
    /// Slot that HSM key uses, can be read with pkcs11-tool
    #[clap(required = false,
        conflicts_with = "private_key_pem",
        long, value_parser=maybe_hex::<u64>, global = true, env = "HSM_SLOT")]
    pub(crate) hsm_slot: Option<u64>,

    /// HSM Key ID, can be read with pkcs11-tool
    #[clap(required = false, conflicts_with = "private_key_pem", long, value_parser=maybe_hex::<u8>, global = true, env = "HSM_KEY_ID")]
    pub(crate) hsm_key_id: Option<u8>,
}

/// HSM authentication arguments
/// These comprise an optional PIN and optional parameters.
/// The PIN is used during autodetection if the optional
/// parameters are missing.
#[derive(ClapArgs, Debug, Clone)]
pub(crate) struct HsmOpts {
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
    pub(crate) hsm_pin: Option<String>,
    #[clap(flatten)]
    pub(crate) hsm_params: HsmParams,
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
    pub(crate) private_key_pem: Option<InputPath>,
    #[clap(flatten)]
    pub(crate) hsm_opts: HsmOpts,
}

impl TryFrom<PathBuf> for AuthOpts {
    type Error = anyhow::Error;

    fn try_from(value: PathBuf) -> std::result::Result<Self, Self::Error> {
        Ok(AuthOpts {
            private_key_pem: Some(InputPath::new(ClioPath::new(value)?)?),
            hsm_opts: HsmOpts {
                hsm_pin: None,
                hsm_params: HsmParams {
                    hsm_slot: None,
                    hsm_key_id: None,
                },
            },
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, author)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) auth_opts: AuthOpts,

    /// Neuron ID
    #[clap(long, global = true, env = "NEURON_ID")]
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

    /// Run the tool offline when possible, i.e., do not sync registry and public dashboard data before the run
    ///
    /// Useful for when the NNS or Public dashboard are unreachable
    #[clap(long)]
    pub offline: bool,

    /// Link to the related forum post, where proposal details can be discussed
    #[clap(long, global = true, visible_aliases = &["forum-link", "forum"])]
    pub forum_post_link: Option<String>,

    /// Path to file which contains cordoned features
    #[clap(long, global = true, visible_aliases = &["cf-file", "cfff"])]
    pub cordon_feature_fallback_file: Option<PathBuf>,
}

// Do not use outside of DRE CLI.
// You can run your command by directly instantiating it.
impl ExecutableCommand for Args {
    fn require_auth(&self) -> AuthRequirement {
        self.subcommands.require_auth()
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        self.subcommands.execute(ctx).await
    }

    /// Validate the command line arguments. You can return an error with something like:
    /// ```rust
    /// if args.neuron_id.is_none() {
    ///    cmd.error(ErrorKind::MissingRequiredArgument, "Neuron ID is required for this command.")).exit();
    /// }
    /// ```
    fn validate(&self, args: &crate::commands::Args, cmd: &mut Command) {
        self.subcommands.validate(args, cmd)
    }
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
            fn require_auth(&self) -> AuthRequirement {
                match &self {
                    $(Subcommands::$var(variant) => variant.require_auth(),)*
                }
            }

            async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
                match &self {
                    $(Subcommands::$var(variant) => variant.execute(ctx).await,)*
                }
            }

            fn validate(&self, args: &crate::commands::Args, cmd: &mut Command) {
                match &self {
                    $(Subcommands::$var(variant) => variant.validate(args, cmd),)*
                }
            }
        }
    }
}
pub(crate) use impl_executable_command_for_enums;

impl_executable_command_for_enums! { DerToPrincipal, Heal, Subnet, Get, Propose, UpdateUnassignedNodes, Version, NodeMetrics, HostOs, Nodes, ApiBoundaryNodes, Vote, Registry, Firewall, Upgrade, Proposals, Completions, Qualify, UpdateAuthorizedSubnets, Neuron }

pub trait ExecutableCommand {
    fn require_auth(&self) -> AuthRequirement;

    fn validate(&self, args: &crate::commands::Args, cmd: &mut Command);

    fn execute(&self, ctx: DreContext) -> impl std::future::Future<Output = anyhow::Result<()>>;
}

#[derive(Clone)]
pub enum AuthRequirement {
    Anonymous, // for get commands
    Signer,    // just authentication details used for signing
    Neuron,    // Signer + neuron_id used for proposals
}

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
