use clap::{Parser, Subcommand};

#[derive(Parser, Clone)]
#[clap(about, version, author)]
pub struct Opts {
    #[clap(long, env = "HSM_PIN")]
    pub(crate) hsm_pin: Option<String>,
    #[clap(long, env = "HSM_SLOT")]
    pub(crate) hsm_slot: Option<String>,
    #[clap(short, long, env = "HSM_KEY_ID")]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(long, env = "PRIVATE_KEY_PEM")]
    pub(crate) private_key_pem: Option<String>,
    #[clap(short, long, env = "NEURON_ID")]
    pub(crate) neuron_id: Option<u64>,
    #[clap(short, long, env = "IC_ADMIN")]
    pub(crate) ic_admin: Option<String>,
    #[clap(
        long,
        env = "BACKEND_URL",
        default_value = "https://dashboard.mercury.dfinity.systems/api/proxy/registry/"
    )]
    pub(crate) backend_url: reqwest::Url,
    #[clap(long, env = "NNS_URL")]
    pub(crate) nns_url: Option<String>,
    #[clap(short, long, env = "DRY_RUN")]
    pub(crate) dry_run: bool,
    #[clap(long, env = "VERBOSE")]
    pub(crate) verbose: bool,

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

            /// Finalize subnet replacements
            #[clap(long)]
            finalize: bool,

            /// Cancel subnet replacements
            #[clap(long)]
            cancel: bool,

            /// Nodes to exclude from available nodes pool
            #[clap(long, takes_value = true, multiple_values = true)]
            exclude: Vec<PrincipalId>,

            /// Nodes to explicitly use for replacement regardless of
            /// decentralization score
            #[clap(long, takes_value = true, multiple_values = true)]
            include: Vec<PrincipalId>,
        },
    }
}
