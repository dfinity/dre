use super::cli_types::*;
use super::types::*;
use anyhow::Error;
use colored::Colorize;
use reqwest::Client;
use std::io::stdin;
use std::process::Command;

use super::state_worker::ReplacementStateWorker;
use std::str;

pub fn add_and_remove_single_node(
    nodes: SubnetReplaceNode,
    cfg: &OperatorConfig,
    worker: &ReplacementStateWorker,
) {
    loop {
        println!("Proposing to take the following actions:");
        println!("{} {}", "Network to be affected:".blue(), "Mainnet".green());
        println!("{} {}", "Subnet(s) affected:".blue(), nodes.subnet);
        println!("{} {}", "Nodes to be removed:".red(), nodes.removed);
        println!("{} {}", "Nodes to be added:".green(), nodes.added);
        println!(
            "{} {}",
            "NNS Proposal URL:".blue(),
            cfg.proposal_url.as_ref().unwrap().clone().green()
        );
        println!("Are you sure you want to continue? Feel free to double check [Y/N]");
        let mut buffer = String::new();
        let stdin = stdin();
        stdin.read_line(&mut buffer).unwrap();
        println!("{}", buffer);
        let buffer = buffer.trim();
        match buffer as &str {
            "Y" => {
                println!("Ic admin functionality not yet implemented");
                let add_output = add_single_node(
                    Subnet {
                        id: nodes.subnet.clone(),
                    },
                    Node { id: nodes.added },
                    cfg,
                );
                worker.add_waited_replacement(add_output, nodes.removed, nodes.subnet);
                break;
            }
            "N" => {
                println!("Cancelling operation");
                break;
            }
            _ => {
                println!("Thou shalt make a choice");
            }
        }
    }
}

#[allow(dead_code)]
async fn add_nodes_to_subnet(
    subnet: Subnet,
    node_count: i32,
    client: &Client,
    url: &str,
    dryrun: DryRun,
) {
    let body = DecentralizedNodeQuery {
        removals: None,
        subnet: subnet.id.clone(),
        node_count,
    };
    let best_nodes = get_decentralized_nodes(url, client, body)
        .await
        .expect("Unable to get nodes from backend")
        .nodes;
    println!("The current best nodes to add are {:?}", best_nodes);
    match dryrun {
        DryRun::True => add_and_remove_nodes(subnet, None, Some(best_nodes)),
        DryRun::False => {
            println!("Not running this command, feel free to double check, if you want to run for real set flag --iwanttodothisforrealipromiseiknowwhatimdoing")
        }
    }
}

#[allow(dead_code)]
pub async fn remove_dead_nodes_from_subnet(
    subnet: Subnet,
    url: &str,
    client: &Client,
    dryrun: DryRun,
) -> Result<(), Error> {
    println!("Not implemented yet (remove_nodes)");
    let nodes_to_remove = get_dead_nodes(subnet.clone(), url, client).await?;
    let assumed_removed = nodes_to_remove.nodes.clone();
    let node_count = assumed_removed.len() as i32;
    let body = DecentralizedNodeQuery {
        subnet: subnet.id.clone(),
        removals: Some(assumed_removed.clone()),
        node_count,
    };
    let best_added_nodes = get_decentralized_nodes(url, client, body).await?.nodes;
    println!(
        "The current dead nodes are {:?}, and the nodes that we would like to add are {:?}",
        assumed_removed, best_added_nodes
    );
    match dryrun {
        DryRun::True => {
            add_and_remove_nodes(subnet, Some(assumed_removed), Some(best_added_nodes));
        }
        DryRun::False => {
            println!("Not running this command, feel free to double check, if you want to run for real set flag --iwanttodothisforrealipromiseiknowwhatimdoing")
        }
    }
    Ok(())
}

pub async fn get_decentralized_nodes(
    url: &str,
    client: &Client,
    params: DecentralizedNodeQuery,
) -> Result<BestNodesResponse, anyhow::Error> {
    let resp = client
        .post(url)
        .json(&params)
        .send()
        .await?
        .json::<BestNodesResponse>()
        .await?;
    Ok(resp)
}

pub async fn get_dead_nodes(
    subnet: Subnet,
    url: &str,
    client: &Client,
) -> Result<NodesToRemoveResponse, anyhow::Error> {
    let resp = client
        .get(url)
        .query(&[("subnet", subnet.id)])
        .send()
        .await?
        .json::<NodesToRemoveResponse>()
        .await?;
    Ok(resp)
}

pub fn add_and_remove_nodes(
    _subnet: Subnet,
    _removed: Option<Vec<String>>,
    _added: Option<Vec<String>>,
) {
}

pub fn remove_single_node(subnet: Subnet, removed: Node, cfg: &OperatorConfig) -> String {
    let subtract_output = Command::new(cfg.ic_admin_cmd.as_ref().unwrap())
        .args([
            "--use-hsm",
            "--slot",
            cfg.hsm_slot.as_ref().unwrap(),
            "--key-id",
            cfg.hsm_key_id.as_ref().unwrap(),
            "--pin",
            cfg.hsm_pin.as_ref().unwrap(),
            "--nns-url",
            cfg.nns_url.as_ref().unwrap(),
            "propose-to-remove-nodes-from-subnet",
            "--subnet",
            &subnet.id,
            "--proposer",
            cfg.neuron_index.as_ref().unwrap(),
            "--proposal-url",
            cfg.proposal_url.as_ref().unwrap(),
            "--summary",
            &format!("Removing {} from subnet.", removed.id),
        ])
        .output()
        .expect("Failed to remove node, panicing");
    str::from_utf8(&subtract_output.stdout)
        .expect("stdout unable to be parsed as text - this should never happen.")
        .to_string()
}

pub fn add_single_node(subnet: Subnet, added: Node, cfg: &OperatorConfig) -> String {
    let add_output = Command::new(cfg.ic_admin_cmd.as_ref().unwrap())
        .args([
            "--use-hsm",
            "--slot",
            cfg.hsm_slot.as_ref().unwrap(),
            "--key-id",
            cfg.hsm_key_id.as_ref().unwrap(),
            "--pin",
            cfg.hsm_pin.as_ref().unwrap(),
            "--nns-url",
            cfg.nns_url.as_ref().unwrap(),
            "propose-to-remove-nodes-from-subnet",
            "--subnet",
            &subnet.id,
            "--proposer",
            cfg.neuron_index.as_ref().unwrap(),
            "--proposal-url",
            cfg.proposal_url.as_ref().unwrap(),
            "--summary",
            &format!("Removing {} from subnet.", added.id),
        ])
        .output()
        .expect("Failed to remove node, panicing");
    str::from_utf8(&add_output.stdout)
        .expect("stdout unable to be parsed as text - this should never happen.")
        .to_string()
}
