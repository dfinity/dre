use crate::cli_types::{NodesVec, SubcmdSubnetUpdateNodes, Subnet};
use anyhow::anyhow;
use colored::Colorize;
use diesel::prelude::SqliteConnection;
use log::debug;
use regex::Regex;

use crate::models;
use crate::utils::{self, env_cfg};
use std::str::FromStr;

fn check_and_submit_proposals_subnet_add_nodes(
    db_connection: &SqliteConnection,
    subnet_id: &String,
) -> Result<(), anyhow::Error> {
    let db_subnet_nodes_to_add = models::subnet_nodes_to_add_get(db_connection, subnet_id);

    for row in &db_subnet_nodes_to_add {
        match (&row.nodes_to_add, &row.nodes_to_remove) {
            (Some(nodes_to_add), Some(nodes_to_remove)) => match row.proposal_add_id {
                None => {
                    let summary_long =
                        proposal_summary_generate(subnet_id, nodes_to_add, nodes_to_remove, 0);
                    let summary_short = proposal_summary_generate_one_line(
                        subnet_id,
                        nodes_to_add,
                        nodes_to_remove,
                        0,
                    );
                    propose_add_nodes_to_subnet(
                        db_connection,
                        subnet_id,
                        nodes_to_add,
                        true,
                        &summary_long,
                        &summary_short,
                    )?;
                }
                Some(add_proposal_id) => {
                    utils::get_proposal_status(add_proposal_id)?;
                    let summary_long = proposal_summary_generate(
                        subnet_id,
                        nodes_to_add,
                        nodes_to_remove,
                        add_proposal_id,
                    );
                    let summary_short = proposal_summary_generate_one_line(
                        subnet_id,
                        nodes_to_add,
                        nodes_to_remove,
                        add_proposal_id,
                    );
                    propose_remove_nodes_from_subnet(
                        db_connection,
                        subnet_id,
                        nodes_to_remove,
                        true,
                        &summary_long,
                        &summary_short,
                    )?;
                }
            },
            _ => {}
        }
    }
    Ok(())
}

pub fn subnet_nodes_replace(
    db_connection: &SqliteConnection,
    nodes: &SubcmdSubnetUpdateNodes,
) -> Result<String, anyhow::Error> {
    debug!(
        "subnet_update_nodes {} nodes: add {:?} remove {:?}",
        nodes.subnet, nodes.nodes_to_add, nodes.nodes_to_remove
    );

    check_and_submit_proposals_subnet_add_nodes(db_connection, &nodes.subnet)?;
    println!(
        "Proposing to update nodes on subnet: {}",
        nodes.subnet.blue()
    );

    match (&nodes.nodes_to_add, &nodes.nodes_to_remove) {
        (Some(nodes_to_add), Some(nodes_to_remove)) => {
            println!("{} {}", "Nodes to add:".green(), nodes_to_add.green());
            println!("{} {}", "Nodes to remove:".red(), nodes_to_remove.red());

            let summary_long =
                proposal_summary_generate(&nodes.subnet, nodes_to_add, nodes_to_remove, 0);
            let summary_short =
                proposal_summary_generate_one_line(&nodes.subnet, nodes_to_add, nodes_to_remove, 0);
            propose_add_nodes_to_subnet(
                db_connection,
                &nodes.subnet,
                nodes_to_add,
                false,
                &summary_long,
                &summary_short,
            )?;

            let summary_long =
                proposal_summary_generate(&nodes.subnet, nodes_to_add, nodes_to_remove, 1);
            let summary_short =
                proposal_summary_generate_one_line(&nodes.subnet, nodes_to_add, nodes_to_remove, 1);
            propose_remove_nodes_from_subnet(
                db_connection,
                &nodes.subnet,
                nodes_to_remove,
                false,
                &summary_long,
                &summary_short,
            )?;

            // Success, user confirmed both operations
            models::subnet_nodes_to_replace_set(
                db_connection,
                &nodes.subnet,
                nodes_to_add,
                nodes_to_remove,
            )?;

            Ok("All done".to_string())
        }
        _ => Err(anyhow!("Please provide both nodes to add and remove")),
    }
}

pub fn propose_add_nodes_to_subnet(
    db_connection: &SqliteConnection,
    subnet_id: &str,
    nodes_to_add: &str,
    real_run: bool,
    summary_long: &str,
    summary_short: &str,
) -> Result<String, anyhow::Error> {
    let subnet: Subnet = Subnet::from_str(subnet_id)?;

    let ic_admin_args = vec![
        "propose-to-add-nodes-to-subnet".to_string(),
        "--subnet".to_string(),
        subnet.id,
        "--proposer".to_string(),
        env_cfg("neuron_id"),
        "--proposal-url".to_string(),
        env_cfg("proposal_url"),
        "--summary".to_string(),
        summary_short.to_string(),
    ];

    if real_run {
        utils::nns_proposals_repo_new_subnet_management(summary_long, summary_short)?;

        let stdout = utils::ic_admin_run(&ic_admin_args, true)?;

        let re = Regex::new(r"(?m)^proposal (\d+)$").unwrap();
        if let Some(cap) = re.captures_iter(stdout.as_str()).next() {
            let proposal_id = cap[1].parse::<i32>().unwrap();
            println!("{}", proposal_id);
            models::subnet_nodes_to_add_update_proposal_id(
                db_connection,
                &subnet_id.to_string(),
                &nodes_to_add.to_string(),
                proposal_id,
            )?;
            return Ok(stdout);
        }
        Err(anyhow!(
            "The proposal number could not be extracted from the output string:\n{}",
            stdout.as_str(),
        ))
    } else {
        // Show the user the command to enqueue and ask for a confirmation
        utils::ic_admin_run(&ic_admin_args, false)?;

        Ok("confirmed".to_string())
    }
}

pub fn propose_remove_nodes_from_subnet(
    db_connection: &SqliteConnection,
    subnet_id: &str,
    nodes_to_remove: &str,
    real_run: bool,
    summary_long: &str,
    summary_short: &str,
) -> Result<String, anyhow::Error> {
    let subnet: Subnet = Subnet::from_str(subnet_id)?;

    let ic_admin_args = vec![
        "propose-to-remove-nodes-from-subnet".to_string(),
        "--subnet".to_string(),
        subnet.id,
        "--proposer".to_string(),
        env_cfg("neuron_id"),
        "--proposal-url".to_string(),
        env_cfg("proposal_url"),
        "--summary".to_string(),
        summary_short.to_string(),
    ];

    if real_run {
        utils::nns_proposals_repo_new_subnet_management(summary_long, summary_short)?;

        let stdout = utils::ic_admin_run(&ic_admin_args, true)?;

        let re = Regex::new(r"(?m)^proposal (\d+)$").unwrap();
        if let Some(cap) = re.captures_iter(stdout.as_str()).next() {
            let proposal_id = cap[1].parse::<i32>().unwrap();
            println!("{}", proposal_id);
            models::subnet_nodes_to_remove_update_proposal_id(
                db_connection,
                &subnet_id.to_string(),
                &nodes_to_remove.to_string(),
                proposal_id,
            )?;
            return Ok(stdout);
        }
        Err(anyhow!(
            "The proposal number could not be extracted from the output string:\n{}",
            stdout.as_str(),
        ))
    } else {
        // Show the user the command to enqueue and ask for a confirmation
        utils::ic_admin_run(&ic_admin_args, false)?;

        Ok("confirmed".to_string())
    }
}

fn proposal_summary_generate(
    subnet_id: &str,
    nodes_to_add: &str,
    nodes_to_remove: &str,
    nodes_to_add_proposal_id: i32,
) -> String {
    let subnet: Subnet = Subnet::from_str(subnet_id).expect("Parsing subnet id failed");
    let nodes_add_vec = NodesVec::from_str(nodes_to_add).expect("parsing nodes_vec failed");
    let nodes_remove_vec = NodesVec::from_str(nodes_to_remove).expect("parsing nodes_vec failed");

    let plural = if nodes_add_vec.len() > 1 || nodes_remove_vec.len() > 1 {
        "nodes"
    } else {
        "a node"
    };

    let step_1_status = if nodes_to_add_proposal_id > 0 {
        format!(
            "proposal [{0}](https://dashboard.internetcomputer.org/proposal/{0})",
            nodes_to_add_proposal_id
        )
    } else {
        "this proposal".to_string()
    };
    let step_2_status = if nodes_to_add_proposal_id > 0 {
        "this"
    } else {
        "upcoming"
    };

    // This formatting will be much nicer when we get fstrings in Rust 2021 https://github.com/rust-lang/rfcs/pull/2795
    format!(
        r#"# Replace {} in subnet {}.

- Step 1 ({}): Add nodes {}
- Step 2 ({} proposal): Remove nodes {}
"#,
        plural,
        subnet.id_short,
        step_1_status,
        nodes_add_vec.as_string_short(),
        step_2_status,
        nodes_remove_vec.as_string_short(),
    )
}

fn proposal_summary_generate_one_line(
    subnet_id: &str,
    nodes_to_add: &str,
    nodes_to_remove: &str,
    nodes_to_add_proposal_id: i32,
) -> String {
    let subnet: Subnet = Subnet::from_str(subnet_id).expect("Parsing subnet id failed");
    let nodes_add_vec = NodesVec::from_str(nodes_to_add).expect("parsing nodes_vec failed");
    let nodes_remove_vec = NodesVec::from_str(nodes_to_remove).expect("parsing nodes_vec failed");

    let plural = if nodes_add_vec.len() > 1 || nodes_remove_vec.len() > 1 {
        "nodes"
    } else {
        "a node"
    };

    let step_1_status = if nodes_to_add_proposal_id > 0 {
        "(done)"
    } else {
        "(this proposal)"
    };
    let step_2_status = if nodes_to_add_proposal_id > 0 {
        "(this proposal)"
    } else {
        "(later)"
    };

    format!(
        "Replace {} in subnet {}: {} add nodes {}; {} remove nodes {}",
        plural,
        subnet.id_short,
        step_1_status,
        nodes_add_vec.as_string_short(),
        step_2_status,
        nodes_remove_vec.as_string_short(),
    )
}

#[cfg(test)]
mod tests;
