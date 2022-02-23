use clap::{Parser, Subcommand};
use std::str::FromStr;

#[derive(Parser, Clone)]
#[clap(about, version, author)]
pub struct Opts {
    #[clap(short = 'p', long, env)]
    pub(crate) hsm_pin: Option<String>,
    #[clap(short = 's', long, env)]
    pub(crate) hsm_slot: Option<String>,
    #[clap(short, long, env)]
    pub(crate) hsm_key_id: Option<String>,
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
    pub(crate) decentralization_url: reqwest::Url,
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
    SubnetReplaceNodes {
        #[clap(short, long)]
        subnet: String,
        #[clap(short = 'a', long = "add")]
        add: Option<String>,
        #[clap(short = 'r', long = "remove")]
        remove: Option<String>,
    },
    DerToPrincipal {
        /// Path to the DER file
        path: String,
    },
    /// Manage an existing subnet
    Subnet(subnet::Cmd),
    Node(node::Cmd),
}

#[derive(Clone)]
pub struct Subnet {
    pub(crate) id: String,
    pub id_short: String,
}

impl FromStr for Subnet {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            id: s.to_string(),
            id_short: s
                .to_string()
                .split_once("-")
                .expect("Could not parse the subnet id.")
                .0
                .to_string(),
        })
    }
}

impl std::fmt::Display for Subnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub id_short: String,
}

impl FromStr for Node {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            id: s.to_string(),
            id_short: s
                .to_string()
                .split_once("-")
                .expect("Could not parse the node id.")
                .0
                .to_string(),
        })
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[derive(Clone)]
pub struct NodesVec(Vec<Node>);

impl std::fmt::Display for NodesVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .clone()
                .into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}

impl FromStr for NodesVec {
    type Err = anyhow::Error;
    fn from_str(nodes_str: &str) -> Result<Self, Self::Err> {
        Ok(NodesVec {
            0: nodes_str
                .replace(",", " ")
                .split(' ')
                .map(|node_id| Node::from_str(node_id).unwrap())
                .collect(),
        })
    }
}

impl NodesVec {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn as_string(&self) -> String {
        format!(
            "[{}]",
            self.0
                .clone()
                .into_iter()
                .map(|e| e.id)
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    pub fn as_string_short(&self) -> String {
        format!(
            "[{}]",
            self.0
                .clone()
                .into_iter()
                .map(|e| e.id_short)
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

pub(crate) mod subnet {
    use super::*;
    use ic_base_types::PrincipalId;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(long, short)]
        pub id: PrincipalId,
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Create a new proposal to rollout a new version to the subnet
        Deploy { version: String },
    }
}

pub(crate) mod node {
    use super::*;
    use ic_base_types::PrincipalId;

    #[derive(Parser, Clone)]
    pub struct Cmd {
        #[clap(subcommand)]
        pub subcommand: Commands,
    }

    #[derive(Subcommand, Clone)]
    pub enum Commands {
        /// Create proposals to replace nodes in the subnet
        Replace { nodes: Vec<PrincipalId> },
    }
}
