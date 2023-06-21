use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use ic_management_types::Network;

#[derive(Parser, Clone)]
#[clap(about, version, author)]
pub struct Opts {
    #[clap(long, env = "HSM_PIN")]
    pub(crate) hsm_pin: Option<String>,
    #[clap(long, value_parser=maybe_hex::<u64>, env = "HSM_SLOT")]
    pub(crate) hsm_slot: Option<u64>,
    #[clap(long, env = "HSM_KEY_ID")]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(long, env = "PRIVATE_KEY_PEM")]
    pub(crate) private_key_pem: Option<String>,
    #[clap(long, env = "NEURON_ID")]
    pub(crate) neuron_id: Option<u64>,
    #[clap(long, env = "IC_ADMIN")]
    pub(crate) ic_admin: Option<String>,
    #[clap(long, env = "DEV")]
    pub(crate) dev: bool,
    #[clap(short, long, env = "YES")]
    pub(crate) yes: bool,
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
    /// Manage replica versions blessing
    Version(version::Cmd),

    /// Manage nodes
    Nodes(nodes::Cmd),
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
            #[clap(long, num_args(1..))]
            min_nakamoto_coefficients: Vec<String>,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Features or Node IDs to only choose from
            #[clap(long, num_args(1..), value_delimiter = ',')]
            only: Vec<String>,

            /// Force the inclusion of the provided nodes for replacement,
            /// regardless of the decentralization score
            #[clap(long, num_args(1..))]
            include: Vec<PrincipalId>,

            /// More verbose execution. For instance, print logs from the
            /// backend.
            #[clap(long)]
            verbose: bool,
        },

        /// Resize the subnet
        Resize {
            // Number of nodes to be added
            #[clap(long)]
            add: usize,

            // Number of nodes to be removed
            #[clap(long)]
            remove: usize,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Features or Node IDs to only choose from
            #[clap(long, num_args(1..), value_delimiter = ',')]
            only: Vec<String>,

            /// Force the inclusion of the provided nodes for replacement,
            /// regardless of the decentralization score
            #[clap(long, num_args(1..))]
            include: Vec<PrincipalId>,

            /// Motivation for resing the subnet
            #[clap(short, long)]
            motivation: Option<String>,

            /// More verbose execution. For instance, print logs from the
            /// backend.
            #[clap(long)]
            verbose: bool,
        },
    }
}

pub(crate) mod version {
    use super::*;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Retire replica versions
        Retire {
            /// Specify if the summary should be edited during the process
            ///
            /// Default value of summary is:
            /// Removing the obsolete IC replica versions from the registry, to
            /// prevent unintended version in the future
            #[clap(long)]
            edit_summary: bool,

            // Simulate submission of the proposal, but do not actually submit it.
            #[clap(long)]
            simulate: bool,
        },

        /// Bless replica version with release notes using the ic-admin CLI and
        /// automatically retire obsolete replica versions
        Update {
            /// Specify the commit hash of the version that is being deployed.
            version: String,

            /// Sepcify the name of the rc branch that contains the release
            /// commits.
            rc_branch_name: String,

            // Simulate submission of the proposal, but do not actually submit it.
            #[clap(long)]
            simulate: bool,
        },
    }
}

pub(crate) mod nodes {
    use super::*;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Remove the nodes from the network
        Remove {
            /// Skip removal of duplicate or dead nodes
            #[clap(long)]
            no_auto: bool,

            /// Specifies the filter used to remove extra nodes
            extra_nodes_filter: Vec<String>,

            /// Motivation for removing additional nodes
            #[clap(long)]
            motivation: Option<String>,
        },
    }
}
