use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use ic_management_types::Network;

#[derive(Parser, Clone)]
#[clap(about, version, author)]
pub struct Opts {
    #[clap(long, env = "HSM_PIN")]
    pub(crate) hsm_pin: Option<String>,
    #[clap(long, parse(try_from_str=maybe_hex), env = "HSM_SLOT")]
    pub(crate) hsm_slot: Option<u64>,
    #[clap(short, long, env = "HSM_KEY_ID")]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(long, env = "PRIVATE_KEY_PEM")]
    pub(crate) private_key_pem: Option<String>,
    #[clap(short, long, env = "NEURON_ID")]
    pub(crate) neuron_id: Option<u64>,
    #[clap(short, long, env = "IC_ADMIN")]
    pub(crate) ic_admin: Option<String>,
    #[clap(long, env = "DEV")]
    pub(crate) dev: bool,
    #[clap(short, long, env = "DRY_RUN")]
    pub(crate) dry_run: bool,
    #[clap(long, env = "VERBOSE")]
    pub(crate) verbose: bool,

    // Specify the target network. Should be either "mainnet" (default) or "staging".
    // If you want to use the cli, use the --nns-url
    #[clap(long, env = "NETWORK", default_value = "mainnet")]
    pub(crate) network: Network,

    #[clap(subcommand)]
    pub(crate) subcommand: Commands,
}

#[derive(Subcommand, Clone)]
pub(crate) enum Commands {
    // Convert a DER file to a Principal
    DerToPrincipal {
        /// Path to the DER file
        path: String,
    },
    /// Manage an existing subnet
    Subnet(subnet::Cmd),
    /// Get a value using ic-admin CLI
    Get {
        /// Arbitrary ic-admin args
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Place a proposal using the ic-admin CLI
    Propose {
        /// Arbitrary ic-admin args
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

pub(crate) mod subnet {
    use super::*;
    use ic_base_types::PrincipalId;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(long, short)]
        pub id: Option<PrincipalId>,
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Create a new proposal to rollout a new version to the subnet
        Deploy { version: String },

        /// Replace the nodes in a subnet
        Replace {
            /// Set of custom nodes to be replaced
            nodes: Vec<PrincipalId>,

            /// Do not replace unhealthy nodes
            #[clap(long)]
            no_heal: bool,

            /// Amount of nodes to be replaced by decentralization optimization
            /// algorithm
            #[clap(short, long)]
            optimize: Option<usize>,

            /// Motivation for replacing custom nodes
            #[clap(short, long)]
            motivation: Option<String>,

            /// Minimum Nakamoto coefficients after the replacement
            #[clap(long, takes_value = true, multiple_values = true)]
            min_nakamoto_coefficients: Vec<String>,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, takes_value = true, multiple_values = true)]
            exclude: Vec<String>,

            /// Force the inclusion of the provided nodes for replacement,
            /// regardless of the decentralization score
            #[clap(long, takes_value = true, multiple_values = true)]
            include: Vec<PrincipalId>,
        },

        /// Extends the size of the subnet
        Extend {
            // Number of nodes to be added
            size: usize,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, takes_value = true, multiple_values = true)]
            exclude: Vec<String>,

            /// Force the inclusion of the provided nodes for replacement,
            /// regardless of the decentralization score
            #[clap(long, takes_value = true, multiple_values = true)]
            include: Vec<PrincipalId>,

            /// Motivation for extending the subnet
            #[clap(short, long)]
            motivation: Option<String>,
        },
    }
}
