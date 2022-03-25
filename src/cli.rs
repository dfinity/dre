use clap::{Parser, Subcommand};

#[derive(Parser, Clone)]
#[clap(about, version, author)]
pub struct Opts {
    #[clap(long, env)]
    pub(crate) hsm_pin: Option<String>,
    #[clap(long, env)]
    pub(crate) hsm_slot: Option<String>,
    #[clap(short, long, env)]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(long, env)]
    pub(crate) private_key_pem: Option<String>,
    #[clap(short, long, env)]
    pub(crate) neuron_id: Option<u64>,
    #[clap(short, long, env)]
    pub(crate) ic_admin: Option<String>,
    #[clap(
        long,
        env,
        default_value = "https://dashboard.mercury.dfinity.systems/api/proxy/registry/"
    )]
    pub(crate) backend_url: reqwest::Url,
    #[clap(long, env)]
    pub(crate) nns_url: Option<String>,
    #[clap(short, long, env)]
    pub(crate) dry_run: bool,
    #[clap(long, env)]
    pub(crate) verbose: bool,

    #[clap(subcommand)]
    pub(crate) subcommand: Commands,
}

#[derive(Subcommand, Clone)]
pub(crate) enum Commands {
    DerToPrincipal {
        /// Path to the DER file
        path: String,
    },
    /// Manage an existing subnet
    Subnet(subnet::Cmd),
    Get {
        /// Arbitrary ic-admin args
        #[clap(allow_hyphen_values = true)]
        args: Vec<String>,
    },
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
            nodes: Vec<PrincipalId>,

            #[clap(long)]
            no_heal: bool,

            #[clap(short, long)]
            optimize: Option<usize>,

            #[clap(short, long)]
            motivation: Option<String>,

            #[clap(long)]
            finalize: bool,
        },
    }
}
