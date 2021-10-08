use anyhow::{anyhow, Error};
use clap::{AppSettings, Clap};
use log::debug;
use std::env;
use std::str::FromStr;

#[derive(Clap, Clone)]
pub struct SubcmdSubnetUpdateNodes {
    #[clap(short, long)]
    pub(crate) subnet: String,
    #[clap(short = 'a', long = "add")]
    pub(crate) nodes_to_add: Option<String>,
    #[clap(short = 'r', long = "remove")]
    pub(crate) nodes_to_remove: Option<String>,
}

impl FromStr for SubcmdSubnetUpdateNodes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Vec<String> = match shlex::split(s) {
            Some(items) => items,
            None => return Err(anyhow!("Invalid input string provided: {}", s)),
        };
        if items.len() != 3 {
            return Err(anyhow!(
                "Three arguments needed: '<subnet_id> <nodes_to_add> <nodes_to_remove>'. Multiple nodes can be provided at once."
            ));
        }
        Ok(Self {
            subnet: items[0].clone(),
            nodes_to_add: Some(items[1].replace(",", " ")),
            nodes_to_remove: Some(items[2].replace(",", " ")),
        })
    }
}

#[derive(Clap, Clone)]
#[clap(version = "0.1")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(short = 'p', long)]
    pub(crate) hsm_pin: Option<String>,
    #[clap(short = 's', long)]
    pub(crate) hsm_slot: Option<String>,
    #[clap(short, long)]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(short, long)]
    pub(crate) neuron_id: Option<String>,
    #[clap(short, long)]
    pub(crate) ic_admin: Option<String>,
    #[clap(short = 'u', long)]
    pub(crate) proposal_url: Option<String>,
    #[clap(short, long)]
    pub(crate) backend_url: Option<String>,
    #[clap(long)]
    pub(crate) nns_url: Option<String>,
    #[clap(short, long)]
    pub(crate) dryrun: bool,
    #[clap(subcommand)]
    pub(crate) subcommand: SubCommand,
}

pub fn load_command_line_config_override(opts: &Opts) {
    if let Some(v) = &opts.hsm_pin {
        env::set_var("hsm_pin", v);
        debug!("override hsm_pin setting with {}", v);
    }
    if let Some(v) = &opts.hsm_slot {
        env::set_var("hsm_slot", v);
        debug!("override hsm_slot setting with {}", v);
    }
    if let Some(v) = &opts.hsm_key_id {
        env::set_var("hsm_key_id", v);
        debug!("override hsm_key_id setting with {}", v);
    }
    if let Some(v) = &opts.neuron_id {
        env::set_var("neuron_id", v);
        debug!("override neuron_id setting with {}", v);
    }
    if let Some(v) = &opts.ic_admin {
        env::set_var("IC_ADMIN", v);
        debug!("override IC_ADMIN setting with {}", v);
    }
    if let Some(v) = &opts.proposal_url {
        env::set_var("proposal_url", v);
        debug!("override proposal_url setting with {}", v);
    }
    if let Some(v) = &opts.backend_url {
        env::set_var("backend_url", v);
        debug!("override backend_url setting with {}", v);
    }
    if let Some(v) = &opts.nns_url {
        env::set_var("nns_url", v);
        debug!("override nns_url setting with {}", v);
    }
}

#[derive(Clap, Clone)]
pub(crate) enum SubCommand {
    #[clap(version = "1.0")]
    SubnetUpdateNodes(SubcmdSubnetUpdateNodes),
    #[clap(version = "1.0")]
    SubnetUpdateNodesRecommended(Subnet),
}

#[derive(Clap, Clone)]
struct UpdateNodesRecommended {
    subnet: Subnet,
}

#[derive(Clap, Clone)]
pub struct Subnet {
    pub(crate) id: String,
    pub id_short: String,
}

impl FromStr for Subnet {
    type Err = Error;
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

#[derive(Clap, Clone, Debug)]
pub struct Node {
    pub id: String,
    pub id_short: String,
}

impl FromStr for Node {
    type Err = Error;
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
    type Err = Error;
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

    pub fn as_vec_string(&self) -> Vec<String> {
        self.0.clone().into_iter().map(|e| e.id).collect::<Vec<String>>()
    }
}
