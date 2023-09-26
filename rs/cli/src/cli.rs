use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use ic_management_types::Network;

// For more info about the version setup, look at https://docs.rs/clap/latest/clap/struct.Command.html#method.version
#[derive(Parser, Clone)]
#[clap(about, version = env!("GIT_HASH"), author)]
pub struct Opts {
    #[clap(long, env = "HSM_PIN", global = true)]
    pub(crate) hsm_pin: Option<String>,
    #[clap(long, value_parser=maybe_hex::<u64>, env = "HSM_SLOT", global = true)]
    pub(crate) hsm_slot: Option<u64>,
    #[clap(long, env = "HSM_KEY_ID", global = true)]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(long, env = "PRIVATE_KEY_PEM", global = true)]
    pub(crate) private_key_pem: Option<String>,
    #[clap(long, env = "NEURON_ID", global = true)]
    pub(crate) neuron_id: Option<u64>,
    #[clap(long, env = "IC_ADMIN", global = true)]
    pub(crate) ic_admin: Option<String>,
    #[clap(long, env = "DEV", global = true)]
    pub(crate) dev: bool,

    // Skip the confirmation prompt
    #[clap(short, long, env = "YES", global = true, conflicts_with = "simulate")]
    pub(crate) yes: bool,

    // Simulate submission of the proposal, but do not actually submit it.
    #[clap(long, aliases = ["dry-run", "dryrun", "no"], global = true, conflicts_with = "yes")]
    pub(crate) simulate: bool,

    #[clap(long, env = "VERBOSE", global = true)]
    pub(crate) verbose: bool,

    // Specify the target network: "mainnet" (default), "staging", or NNS URL
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
            #[clap(long, num_args(1..))]
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
            #[clap(long, num_args(1..))]
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

        /// Create a new subnet
        Create {
            /// Number of nodes in the subnet
            #[clap(long, default_value_t = 13)]
            size: usize,

            /// Minimum nakamoto coefficients desired
            #[clap(long, num_args(1..))]
            min_nakamoto_coefficients: Vec<String>,

            /// Features or Node IDs to exclude from the available nodes pool
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Features or Node IDs to only choose from
            #[clap(long, num_args(1..))]
            only: Vec<String>,

            /// Force the inclusion of the provided nodes,
            /// regardless of the decentralization score
            #[clap(long, num_args(1..))]
            include: Vec<PrincipalId>,

            /// Motivation for creating the subnet
            #[clap(short, long)]
            motivation: Option<String>,

            /// More verbose execution. For instance, print logs from the
            /// backend.
            #[clap(long)]
            verbose: bool,

            #[clap(long)]
            replica_version: Option<String>,
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
        /// Update the elected/blessed replica versions in the registry
        /// by adding a new version and potentially removing obsolete versions
        Update {
            /// Specify the commit hash of the version that is being elected.
            version: String,

            /// Git tag for the release.
            release_tag: String,
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

            /// Features or Node IDs to not remove (exclude from the removal)
            #[clap(long, num_args(1..))]
            exclude: Vec<String>,

            /// Motivation for removing additional nodes
            #[clap(long)]
            motivation: Option<String>,
        },
    }
}
