use clap::{Clap, AppSettings};
use std::str::FromStr;
use anyhow::{anyhow, Error};
use serde::{Serialize, Deserialize};

#[allow(dead_code)]
pub enum DryRun {
    False,
    True
}

#[derive(Clap, Clone)]
pub struct SingleNode {
    #[clap(short, long)]
    pub(crate) subnet: String,
    #[clap(short, long)]
    pub(crate) removed: String,
    #[clap(short, long)]
    pub(crate) added: String
}

impl FromStr for SingleNode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Vec<&str> = s.split(' ').collect();
        if items.len() != 3 {
            return Err(anyhow!("Three arguments needed, in order <subnet_id> <node_removed_id> <node_added_id>"))
        }
        Ok(Self{
            subnet: items[0].to_string(),
            removed: items[1].to_string(),
            added: items[2].to_string()
        })
    }
}

#[derive(Clap, Clone)]
pub struct MultipleNodes {
    subnet: String,
    removed: Vec<String>,
    added: Vec<String>
}

impl FromStr for MultipleNodes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Vec<&str> = s.split(' ').collect();
        if items.len() < 5 {
            return Err(anyhow!("Pattern for lists of arbitrary nodes is <subnet_id> <added/removed> <node1> <node2> <added/removed> <node1> <node2>"));
        }
        let subnet = items[0].to_string();
        let mut removed: Vec<String> = Vec::new();
        let mut added: Vec<String> = Vec::new();
        let mut flag = true;
        for node in items {
            if node == "removed" {
                flag = false;
            } else if node == "added" {
                flag = true
            } else {
                if flag == true {
                    added.push(node.to_string());
                } else {
                    removed.push(node.to_string());
                }
            }
        }
        Ok(Self{
            subnet,
            removed,
            added
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct OperatorConfig {
    pub(crate) hsm_pin: Option<String>,
    pub(crate) hsm_slot: Option<String>,
    pub(crate) hsm_key_id: Option<String>,
    pub(crate) neuron_index: Option<String>,
    pub(crate) proposal_url: Option<String>,
    pub(crate) ic_admin_cmd: Option<String>,
    pub(crate) nns_url: Option<String>,
    pub(crate) dryrun: Option<bool>,
    pub(crate) backend_url: Option<String>
}

#[derive(Clap, Clone)]
#[clap(version = "1.0", author = "Connor Matza <coic_admin_cmdnnor.matza@dfinity.org>")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(short='p', long)]
    pub(crate) hsm_pin: Option<String>,
    #[clap(short='s', long)]
    pub(crate) hsm_slot: Option<String>,
    #[clap(short, long)]
    pub(crate) hsm_key_id: Option<String>,
    #[clap(short, long)]
    pub(crate) neuron_index: Option<String>,
    #[clap(short, long)]
    pub(crate) ic_admin_cmd: Option<String>,
    #[clap(short='u', long)]
    pub(crate) proposal_url: Option<String>,
    #[clap(short, long)]
    pub(crate) backend_url: Option<String>,
    #[clap(short, long)]
    pub(crate) dryrun: bool,
    #[clap(subcommand)]
    pub(crate) subcommand: SubCommand,
    #[clap(long)]
    pub(crate) nns_url: Option<String>
}

#[derive(Clap, Clone)]
pub(crate) enum SubCommand {
    #[clap(version = "1.0",  author = "Someone")]
    ReplaceSingleArbitrary(SingleNode),
    #[clap(version = "1.0",  author = "Someone")]
    ReplaceRecommended(Subnet),
    #[clap(version = "1.0",  author = "Someone")]
    AddNodesRecommended(Subnet),
    #[clap(version = "1.0",  author = "Someone")]
    AddNodesArbitrary(Nodes),
    #[clap(version = "1.0",  author = "Someone")]
    ReplaceBatchArbitrary(MultipleNodes)
}

#[derive(Clap, Clone)]
struct ReplaceRecommended {
    subnet: Subnet
}

#[derive(Clap, Clone)]
struct AddNodesRecommended {
    subnet: Subnet,
}

#[derive(Clap, Clone)]
struct AddNodesArbitrary {
    subnet: Subnet, 
    nodes: Nodes
}

#[derive(Clap, Clone)]
struct ReplaceBatchArbitrary {
    replacements: MultipleNodes
}

#[derive(Clap, Clone)]
pub struct Subnet {
    pub(crate) id: String,
}

#[derive(Clap, Clone)]
pub struct Node {
    pub id: String,
}

impl FromStr for Subnet {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self{id: s.to_string()})
    }
}

impl FromStr for Node {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self{id: s.to_string()})
    }
}

#[derive(Clap, Clone)]
pub struct Nodes {
    subnet: String,
    list: Vec<Node>
}

impl FromStr for Nodes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Nodes{list: s.split(' ').skip(1).map(|x| Node{id: x.to_string()}).collect(), subnet: s.split(' ').next().unwrap().to_string()})
    }
}

pub enum ReplaceStateMachine {
    Add,
    Remove
}