use crate::cli_types::{NodesVec, SubcmdSubnetUpdateNodes, Subnet};
use crate::types::*;
use anyhow::{anyhow, Error};
use colored::Colorize;
use diesel::prelude::SqliteConnection;
use log::{debug, info, warn};
use python_input::input;
use regex::Regex;
use reqwest::Client;
use std::process::Command;

use crate::models;
use crate::utils::env_cfg;
use std::str;
use std::str::FromStr;

fn check_and_submit_proposals_subnet_add_nodes(
    db_connection: &SqliteConnection,
    subnet_id: &String,
) -> Result<(), anyhow::Error> {
    let db_subnet_nodes_to_add = models::subnet_nodes_to_add_get(db_connection, subnet_id);

    for row in &db_subnet_nodes_to_add {
        match &row.nodes_to_add {
            Some(nodes_to_add) => {
                if row.proposal_id_for_add.is_none() {
                    propose_add_nodes_to_subnet(db_connection, subnet_id, nodes_to_add, true)?;
                }
            }
            None => {}
        }
    }
    Ok(())
}

pub fn subnet_update_nodes(
    db_connection: &SqliteConnection,
    nodes: &SubcmdSubnetUpdateNodes,
) -> Result<String, anyhow::Error> {
    debug!(
        "subnet_update_nodes {} nodes: add {:?} remove {:?}",
        nodes.subnet, nodes.nodes_to_add, nodes.nodes_to_remove
    );

    check_and_submit_proposals_subnet_add_nodes(&db_connection, &nodes.subnet)?;
    println!(
        "Proposing to update nodes on subnet: {}",
        nodes.subnet.blue()
    );

    match &nodes.nodes_to_add {
        Some(nodes_to_add) => {
            println!("{} {}", "Nodes to add to the subnet:".green(), nodes_to_add);
            propose_add_nodes_to_subnet(db_connection, &nodes.subnet, nodes_to_add, false)?;
        }
        None => {}
    };

    match &nodes.nodes_to_remove {
        Some(nodes_to_remove) => {
            println!(
                "{} {}",
                "Nodes to remove from the subnet:".green(),
                nodes_to_remove
            );
            propose_remove_nodes_from_subnet(&db_connection, &nodes.subnet, nodes_to_remove)?;
        }
        None => {}
    };

    Ok("All done".to_string())
}

#[allow(dead_code)]
async fn add_recommended_nodes_to_subnet(
    subnet: Subnet,
    node_count: i32,
    client: &Client,
    url: &str,
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
}

#[allow(dead_code)]
pub async fn remove_dead_nodes_from_subnet(
    subnet: Subnet,
    url: &str,
    client: &Client,
    // dryrun: DryRun,
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

fn ic_admin_run(args: &Vec<String>, confirmed: bool) -> Result<String, Error> {
    let args_basic = vec![
        "--use-hsm".to_string(),
        "--slot".to_string(),
        env_cfg("hsm_slot"),
        "--key-id".to_string(),
        env_cfg("hsm_key_id"),
        "--pin".to_string(),
        env_cfg("hsm_pin"),
        "--nns-url".to_string(),
        env_cfg("nns_url"),
    ];

    let ic_admin_args = [args_basic, args.clone()].concat();

    if confirmed {
        info!("Real run of the ic-admin command");
        let output = Command::new(env_cfg("ic_admin_bin_path"))
            .args(ic_admin_args)
            .output()?;
        let stdout = String::from_utf8_lossy(output.stdout.as_ref()).to_string();
        info!("STDOUT:\n{}", stdout);
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(output.stderr.as_ref()).to_string();
            warn!("STDERR:\n{}", stderr);
        }
        Ok(stdout)
    } else {
        info!("Dry-run of the ic-admin command execution");
        // Show the user the line that would be executed and let them decide if they want to proceed.
        println!(
            "{}",
            "WARNING: Will execute the following command, please confirm.".red(),
        );

        println!(
            "$ {} {}",
            env_cfg("ic_admin_bin_path").green(),
            shlex::join(
                ic_admin_args
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
            )
            .green()
        );

        let buffer = input("Are you sure you want to continue? [y/N] ");
        match buffer.to_uppercase().as_str() {
            "Y" | "YES" => Ok("User confirmed".to_string()),
            _ => Err(anyhow!(
                "Cancelling operation, user entered '{}'",
                buffer.as_str(),
            )),
        }
    }
}

pub fn propose_add_nodes_to_subnet(
    db_connection: &SqliteConnection,
    subnet_id: &String,
    nodes_to_add: &String,
    confirmed: bool,
) -> Result<String, anyhow::Error> {
    let subnet = Subnet {
        id: subnet_id.clone(),
    };
    let nodes_vec = NodesVec::from_str(nodes_to_add.as_str()).expect("parsing nodes_vec failed");

    let ic_admin_args = vec![
        "propose-to-add-nodes-to-subnet".to_string(),
        "--subnet".to_string(),
        subnet.id,
        "--proposer".to_string(),
        env_cfg("neuron_id"),
        "--proposal-url".to_string(),
        env_cfg("proposal_url"),
        "--summary".to_string(),
        format!("Add nodes to subnet: {}", nodes_vec),
    ];

    if !confirmed {
        // Do a dry-run and see if the user confirms the operation
        ic_admin_run(&ic_admin_args, false)?;

        // Dry-run was a success, user confirmed
        models::subnet_nodes_to_add_set(db_connection, subnet_id, &nodes_to_add)?;
    }

    let stdout = ic_admin_run(&ic_admin_args, true)?;

    let re = Regex::new(r"(?m)^proposal (\d+)$").unwrap();
    for cap in re.captures_iter(stdout.as_str()) {
        let proposal_id = cap[1].parse::<i32>().unwrap();
        println!("{}", proposal_id);
        models::subnet_nodes_to_add_update_proposal_id(
            db_connection,
            subnet_id,
            nodes_to_add,
            proposal_id,
        )?;
        return Ok(stdout);
    }
    Err(anyhow!(
        "The proposal number could not be extracted from the output string:\n{}",
        stdout.as_str(),
    ))
}

pub fn propose_remove_nodes_from_subnet(
    db_connection: &SqliteConnection,
    subnet_id: &String,
    nodes_to_remove: &String,
) -> Result<String, anyhow::Error> {
    let subnet = Subnet {
        id: subnet_id.clone(),
    };
    let nodes_vec = NodesVec::from_str(nodes_to_remove.as_str()).expect("parsing nodes_vec failed");
    let ic_admin_args = vec![
        "propose-to-remove-nodes-from-subnet".to_string(),
        "--subnet".to_string(),
        subnet.id,
        "--proposer".to_string(),
        env_cfg("neuron_id"),
        "--proposal-url".to_string(),
        env_cfg("proposal_url"),
        "--summary".to_string(),
        format!("Remove nodes to subnet: {}", nodes_vec),
    ];
    ic_admin_run(&ic_admin_args, false)
}
