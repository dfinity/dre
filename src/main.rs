use confy::load;
use serde::{Serialize, Deserialize};
use clap::{AppSettings, Clap};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use anyhow::{anyhow, Error};
use std::fmt::Display;
use futures::executor::block_on;
use std::str::FromStr;
use std::process::Command;
use colored::Colorize;
use std::io::{self, Read};
mod types;


#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct OperatorConfig {
    hsm_pin: Option<String>,
    hsm_slot: Option<String>,
    hsm_key_id: Option<String>,
    neuron_index: Option<String>,
    proposal_url: Option<String>,
    ic_admin_cmd: Option<String>,
    nns_url: Option<String>,
    dryrun: Option<bool>,
    backend_url: Option<String>
}

#[derive(Clap)]
#[clap(version = "1.0", author = "Connor Matza <coic_admin_cmdnnor.matza@dfinity.org>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short='p', long)]
    hsm_pin: Option<String>,
    #[clap(short='s', long)]
    hsm_slot: Option<String>,
    #[clap(short, long)]
    hsm_key_id: Option<String>,
    #[clap(short, long)]
    neuron_index: Option<String>,
    #[clap(short, long)]
    ic_admin_cmd: Option<String>,
    #[clap(short='u', long)]
    proposal_url: Option<String>,
    #[clap(short, long)]
    backend_url: Option<String>,
    #[clap(short, long)]
    dryrun: bool,
    #[clap(subcommand)]
    subcommand: SubCommand,
    #[clap(long)]
    nns_url: Option<String>
}

#[derive(Clap)]
enum SubCommand {
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

#[derive(Clap)]
struct ReplaceRecommended {
    subnet: Subnet
}

#[derive(Clap)]
struct AddNodesRecommended {
    subnet: Subnet,
}

#[derive(Clap)]
struct AddNodesArbitrary {
    subnet: Subnet, 
    nodes: Nodes
}

#[derive(Clap)]
struct ReplaceBatchArbitrary {
    replacements: MultipleNodes
}

#[derive(Clap, Clone)]
struct SingleNode {
    #[clap(short, long)]
    subnet: String,
    #[clap(short, long)]
    removed: String,
    #[clap(short, long)]
    added: String
}

impl FromStr for SingleNode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Vec<&str> = s.split(" ").collect();
        if items.len() != 3 as usize {
            return Err(anyhow!("Three arguments needed, in order <subnet_id> <node_removed_id> <node_added_id>"))
        }
        Ok(Self{
            subnet: items[0].to_string(),
            removed: items[1].to_string(),
            added: items[2].to_string()
        })
    }
}

#[derive(Clap)]
struct MultipleNodes {
    subnet: String,
    removed: Vec<String>,
    added: Vec<String>
}

impl FromStr for MultipleNodes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let items: Vec<&str> = s.split(" ").collect();
        if items.len() < 5 as usize {
            return Err(anyhow!("Pattern for lists of arbitrary nodes is <subnet_id> <added/removed> <node1> <node2> <added/removed> <node1> <node2>"));
        }
        let subnet = items[0].to_string();
        let mut removed: Vec<String> = Vec::new();
        let mut added: Vec<String> = Vec::new();
        let mut flag = true;
        for val in 1..items.len() {
            let node = items[val];
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

#[derive(Clap, Clone)]
pub struct Subnet {
    id: String,
}

#[derive(Clap, Debug)]
struct Node {
    id: String,
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

#[derive(Clap)]
struct Nodes {
    subnet: String,
    list: Vec<Node>
}

impl FromStr for Nodes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Nodes{list: s.split(' ').skip(1).map(|x| Node{id: x.to_string()}).collect(), subnet: s.split(' ').next().unwrap().to_string()})
    }
}

fn main() {
    let client = reqwest::Client::new();
    let opts = Opts::parse();
    let mut cfg: OperatorConfig = confy::load_path("management_config.toml").unwrap();
    cfg = merge_opts_into_cfg(&opts, cfg);
    match &opts.subcommand {
        SubCommand::ReplaceSingleArbitrary(v) => { add_and_remove_single_node(v.clone(), &opts, cfg)},
        _ => { println!("Not implemented yet")}
    }
}

async fn add_nodes_to_subnet(subnet: Subnet, node_count: i32, client: &Client, url: &str, dryrun: types::DryRun) {
    let body = types::DecentralizedNodeQuery{
        removals: None,
        subnet: subnet.id.clone(),
        node_count
    };
    let best_nodes = get_decentralized_nodes(url, client, body).await.expect("Unable to get nodes from backend").nodes.clone();
    println!("The current best nodes to add are {:?}", best_nodes);
    match dryrun {
        types::DryRun::True => {
            add_and_remove_nodes(subnet, None, Some(best_nodes))
            },
        types::DryRun::False => {
            println!("Not running this command, feel free to double check, if you want to run for real set flag --iwanttodothisforrealipromiseiknowwhatimdoing")
        }
        }
    }

async fn remove_dead_nodes_from_subnet(subnet: Subnet, url: &str, client: &Client, dryrun: types::DryRun) -> Result<(), Error> {
    println!("Not implemented yet (remove_nodes)");
    let nodes_to_remove = get_dead_nodes(subnet.clone(), url, client).await?;
    let assumed_removed = nodes_to_remove.nodes.clone();
    let node_count = assumed_removed.len() as i32;
    let body = types::DecentralizedNodeQuery{
        subnet: subnet.id.clone(),
        removals: Some(assumed_removed.clone()),
        node_count
    };
    let best_added_nodes = get_decentralized_nodes(url, client, body).await?.nodes.clone();
    println!("The current dead nodes are {:?}, and the nodes that we would like to add are {:?}", assumed_removed, best_added_nodes);
    match dryrun {
        types::DryRun::True => {
            add_and_remove_nodes(subnet, Some(assumed_removed), Some(best_added_nodes));
        },
        types::DryRun::False => {
            println!("Not running this command, feel free to double check, if you want to run for real set flag --iwanttodothisforrealipromiseiknowwhatimdoing")
        }
    }
    Ok(())
}

async fn get_decentralized_nodes(url: &str, client: &Client, params: types::DecentralizedNodeQuery) -> Result<types::BestNodesResponse, anyhow::Error> {
    let resp = client.post(url).json(&params).send().await?.json::<types::BestNodesResponse>().await?;
    Ok(resp)
}

async fn get_dead_nodes(subnet: Subnet, url: &str, client: &Client) -> Result<types::NodesToRemoveResponse, anyhow::Error> {
    let resp = client.get(url).query(&[("subnet", subnet.id)]).send().await?.json::<types::NodesToRemoveResponse>().await?;
    Ok(resp)
}

pub fn add_and_remove_nodes(subnet: Subnet, removed: Option<Vec<String>>, added: Option<Vec<String>>) {

}

fn add_and_remove_single_node(nodes: SingleNode, opts: &Opts, cfg: OperatorConfig) {
    loop {
    println!("{}", "Proposing to take the following actions:");
    println!("{} {}", "Network to be affected:".blue(), "Mainnet");
    println!("{} {}", "Subnet(s) affected:".blue(), nodes.subnet);
    println!("{} {}", "Nodes to be removed:".red(), nodes.removed);
    println!("{} {}", "Nodes to be added:".green(), nodes.added);
    println!("{} {}", "NNS Proposal URL:", cfg.proposal_url.as_ref().unwrap().clone());
    println!("{}", "Are you sure you want to continue? Feel free to double check [Y/N]");
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_line(&mut buffer).unwrap();
    println!("{}", buffer);
    let buffer = buffer.trim();
    match &buffer as &str { 
        "Y" => { println!("Ic admin functionality not yet implemented");
                let add_output = Command::new(cfg.ic_admin_cmd.as_ref().unwrap())
                    .args(["--use-hsm", "--slot", &cfg.hsm_slot.as_ref().unwrap(), "--key-id", &cfg.hsm_key_id.as_ref().unwrap(), "--pin", &cfg.hsm_pin.as_ref().unwrap(), "--nns-url", &cfg.nns_url.as_ref().unwrap(), 
                            "propose-to-add-nodes-to-subnet",  "--subnet", &nodes.subnet, "--proposer", &cfg.neuron_index.as_ref().unwrap(), "--proposal-url", &cfg.proposal_url.as_ref().unwrap(), "--summary", &format!("Adding {} to subnet {} to replace {}", nodes.added, nodes.subnet, nodes.removed), &nodes.added])
                    .output()
                    .expect("Failed to add node, panicing");
                let subtract_output = Command::new(cfg.ic_admin_cmd.as_ref().unwrap())
                    .args(["--use-hsm", "--slot", &cfg.hsm_slot.as_ref().unwrap(), "--key-id", &cfg.hsm_key_id.as_ref().unwrap(), "--pin", &cfg.hsm_pin.as_ref().unwrap(), "--nns-url", &cfg.nns_url.as_ref().unwrap(), 
                            "propose-to-remove-nodes-from-subnet",  "--subnet", &nodes.subnet, "--proposer", &cfg.neuron_index.as_ref().unwrap(), "--proposal-url", &cfg.proposal_url.as_ref().unwrap(), "--summary", &format!("Removing {} from subnet {} replced by {}", nodes.added, nodes.subnet, nodes.removed), &nodes.removed])
                    .output()
                    .expect("Failed to remove node, panicing");
                println!("Operation successful");


    
                break;
            },
        "N" => { println!("Cancelling operation");
                 break;
                },
        _ => { println!("Thou shalt make a choice"); }
    }
    }


}

fn merge_opts_into_cfg(opts: &Opts, mut cfg: OperatorConfig) -> OperatorConfig {
    match &opts.backend_url {
        Some(v) => { cfg.backend_url = Some(v.clone()) },
        None => (),
    }
    match &opts.nns_url {
        Some(v) => { cfg.nns_url = Some(v.clone()) },
        None => (),
    }
    match &opts.hsm_pin {
        Some(v) => { cfg.hsm_pin = Some(v.clone()) },
        None => (),
    }
    match &opts.hsm_slot {
        Some(v) => { cfg.hsm_slot = Some(v.clone()) },
        None => (),
    }
    match &opts.hsm_key_id {
        Some(v) => { cfg.hsm_key_id = Some(v.clone()) },
        None => (),
    }
    match &opts.neuron_index {
        Some(v) => { cfg.neuron_index = Some(v.clone()) },
        None => (),
    }
    match &opts.ic_admin_cmd {
        Some(v) => { cfg.ic_admin_cmd = Some(v.clone()) },
        None => (),
    }
    match &opts.proposal_url {
        Some(v) => { cfg.proposal_url = Some(v.clone()) },
        None => (),
    }
    cfg
}