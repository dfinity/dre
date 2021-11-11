use crate::cli_types::{NodesVec, SubcmdSubnetUpdateNodes, Subnet};
use anyhow::anyhow;
use colored::Colorize;
use diesel::prelude::SqliteConnection;
use log::{debug, info};
use regex::Regex;

use crate::model_proposals;
use crate::model_subnet_update_nodes;
use crate::utils::{self, env_cfg};
use std::str::FromStr;

fn proposal_check_if_completed(
    db_connection: &SqliteConnection,
    proposal_id: Option<i32>,
) -> Result<Option<utils::ProposalStatus>, anyhow::Error> {
    match proposal_id {
        Some(proposal_id) => {
            let proposal = utils::get_proposal_status(proposal_id)?;
            if proposal.executed_timestamp_seconds > 0 {
                model_proposals::proposal_set_executed(
                    db_connection,
                    proposal_id,
                    proposal.executed_timestamp_seconds,
                )?;
            }
            if proposal.failed_timestamp_seconds > 0 {
                model_proposals::proposal_set_failed(
                    db_connection,
                    proposal_id,
                    proposal.failed_timestamp_seconds,
                    &proposal.failure_reason,
                )?;
            }
            Ok(Some(proposal))
        }
        None => Ok(None),
    }
}

pub fn check_and_submit_proposals_subnet_add_nodes(
    db_connection: &SqliteConnection,
    subnet_id: &str,
) -> Result<bool, anyhow::Error> {
    let rows = model_subnet_update_nodes::subnet_rows_in_progress_get(db_connection, subnet_id);

    let mut pending = false;
    for row in &rows {
        match (&row.nodes_to_add, &row.nodes_to_remove) {
            (Some(nodes_to_add), Some(nodes_to_remove)) => {
                if !model_proposals::is_proposal_executed(db_connection, row.proposal_add_id) {
                    proposal_check_if_completed(db_connection, row.proposal_add_id)?;
                }
                if !model_proposals::is_proposal_executed(db_connection, row.proposal_remove_id) {
                    proposal_check_if_completed(db_connection, row.proposal_remove_id)?;
                }
                if model_proposals::is_proposal_executed(db_connection, row.proposal_add_id) {
                    if model_proposals::is_proposal_executed(db_connection, row.proposal_remove_id) {
                        model_subnet_update_nodes::subnet_row_mark_completed(db_connection, row);
                        continue;
                    }
                }
                match row.proposal_add_id {
                    None => {
                        info!("No proposal yet for adding nodes {}", nodes_to_add);
                        // No proposal submitted yet, let's do that now
                        let proposal_summary = proposal_generate_summary(subnet_id, nodes_to_add, nodes_to_remove, 0);
                        let proposal_title = proposal_generate_title(subnet_id, nodes_to_add, nodes_to_remove, 0);

                        let ic_admin_stdout = propose_add_nodes_to_subnet(
                            subnet_id,
                            nodes_to_add,
                            true,
                            &proposal_title,
                            &proposal_summary,
                        )?;
                        info!("Proposal submitted successfully: {}", ic_admin_stdout);
                        let proposal_id = parse_proposal_id_from_ic_admin_stdout(ic_admin_stdout.as_str())?;
                        model_proposals::proposal_add(
                            db_connection,
                            proposal_id,
                            &proposal_title,
                            &proposal_summary,
                            &ic_admin_stdout,
                        );
                        model_subnet_update_nodes::subnet_nodes_to_add_update_proposal_id(
                            db_connection,
                            &subnet_id.to_string(),
                            &nodes_to_add.to_string(),
                            proposal_id,
                        )?;
                    }
                    Some(add_proposal_id) => {
                        // Proposal already submitted
                        if model_proposals::is_proposal_executed(db_connection, row.proposal_add_id) {
                            // Add proposal already marked finished execution
                            if row.proposal_remove_id.is_none() {
                                // remove proposal was not created yet
                                info!("Proposal for adding nodes {} was already marked finished, proceeding with removal of {}", nodes_to_add, nodes_to_remove);
                                let proposal = utils::get_proposal_status(add_proposal_id)?;
                                if proposal.executed_timestamp_seconds > 0 {
                                    proposal_delete_create(
                                        db_connection,
                                        add_proposal_id,
                                        subnet_id,
                                        nodes_to_add,
                                        nodes_to_remove,
                                    )?;
                                }
                            }
                        } else {
                            let proposal = proposal_check_if_completed(db_connection, row.proposal_add_id)?;
                            if let Some(proposal) = proposal {
                                if proposal.executed_timestamp_seconds > 0 {
                                    proposal_delete_create(
                                        db_connection,
                                        add_proposal_id,
                                        subnet_id,
                                        nodes_to_add,
                                        nodes_to_remove,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        pending = true;
    }
    Ok(pending)
}

fn proposal_delete_create(
    db_connection: &SqliteConnection,
    add_proposal_id: i32,
    subnet_id: &str,
    nodes_to_add: &str,
    nodes_to_remove: &str,
) -> Result<(), anyhow::Error> {
    info!(
        "Proposal {} for adding nodes was successfully executed, now remove the nodes",
        add_proposal_id
    );
    let proposal_title = proposal_generate_title(subnet_id, nodes_to_add, nodes_to_remove, add_proposal_id);
    let proposal_summary = proposal_generate_summary(subnet_id, nodes_to_add, nodes_to_remove, add_proposal_id);

    let ic_admin_stdout = propose_remove_nodes_from_subnet(nodes_to_remove, true, &proposal_title, &proposal_summary)?;
    info!("Proposal submitted successfully: {}", ic_admin_stdout);
    let proposal_id = parse_proposal_id_from_ic_admin_stdout(ic_admin_stdout.as_str())?;
    model_proposals::proposal_add(
        db_connection,
        proposal_id,
        &proposal_title,
        &proposal_summary,
        &ic_admin_stdout,
    );
    model_subnet_update_nodes::subnet_nodes_to_remove_update_proposal_id(
        db_connection,
        &subnet_id.to_string(),
        &nodes_to_remove.to_string(),
        proposal_id,
    )?;
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
    println!("Proposing to update nodes on subnet: {}", nodes.subnet.blue());

    match (&nodes.nodes_to_add, &nodes.nodes_to_remove) {
        (Some(nodes_to_add), Some(nodes_to_remove)) => {
            for row in model_subnet_update_nodes::subnet_rows_get(db_connection, &nodes.subnet) {
                if row.nodes_to_add == nodes.nodes_to_add && row.nodes_to_remove == nodes.nodes_to_remove {
                    let proposal_add = proposal_check_if_completed(db_connection, row.proposal_add_id)?;
                    let proposal_remove = proposal_check_if_completed(db_connection, row.proposal_remove_id)?;
                    if let (Some(proposal_add), Some(proposal_remove)) = (&proposal_add, &proposal_remove) {
                        let add_finished =
                            proposal_add.executed_timestamp_seconds > 0 || proposal_add.failed_timestamp_seconds > 0;
                        let remove_finished = proposal_remove.executed_timestamp_seconds > 0
                            || proposal_remove.failed_timestamp_seconds > 0;
                        if add_finished && remove_finished {
                            info!("Both add and remove finished on subnet {}", nodes.subnet);
                            return Ok("Nothing to do.".to_owned());
                        }
                    }
                    info!("Subnet changes are already pending");
                    info!("Proposal to add: {:?}", proposal_add);
                    info!("Proposal to remove: {:?}", proposal_remove);
                    return Ok("Changes already in progress".to_string());
                } else {
                    info!("Subnet already has different changes pending: {:?}", row);
                    return Err(anyhow!("Subnet already has different changes pending"));
                }
            }

            let nodes_add_vec = NodesVec::from_str(nodes_to_add).expect("parsing nodes_vec failed");
            let nodes_rm_vec = NodesVec::from_str(nodes_to_remove).expect("parsing nodes_vec failed");
            println!(
                "{} {}",
                "Nodes to add:".green(),
                nodes_add_vec.as_string_short().green()
            );
            println!("{} {}", "Nodes to remove:".red(), nodes_rm_vec.as_string_short().red());

            let proposal_title = proposal_generate_title(&nodes.subnet, nodes_to_add, nodes_to_remove, 0);
            let proposal_summary = proposal_generate_summary(&nodes.subnet, nodes_to_add, nodes_to_remove, 0);
            propose_add_nodes_to_subnet(&nodes.subnet, nodes_to_add, false, &proposal_title, &proposal_summary)?;

            let proposal_title = proposal_generate_title(&nodes.subnet, nodes_to_add, nodes_to_remove, 1);
            let proposal_summary = proposal_generate_summary(&nodes.subnet, nodes_to_add, nodes_to_remove, 1);
            propose_remove_nodes_from_subnet(nodes_to_remove, false, &proposal_title, &proposal_summary)?;

            // Success, user confirmed both operations
            model_subnet_update_nodes::subnet_nodes_to_replace_set(
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

fn parse_proposal_id_from_ic_admin_stdout(text: &str) -> Result<i32, anyhow::Error> {
    let re = Regex::new(r"(?m)^proposal (\d+)$").unwrap();
    let mut captures = re.captures_iter(text);
    if let Some(cap) = captures.next() {
        let proposal_id = cap[1].parse::<i32>().unwrap();
        Ok(proposal_id)
    } else {
        Err(anyhow!(
            "The proposal number could not be extracted from the text:\n{}",
            text,
        ))
    }
}

pub fn propose_add_nodes_to_subnet(
    subnet_id: &str,
    nodes_to_add: &str,
    real_run: bool,
    proposal_title: &str,
    proposal_summary: &str,
) -> Result<String, anyhow::Error> {
    let subnet: Subnet = Subnet::from_str(subnet_id)?;
    let nodes_vec = NodesVec::from_str(nodes_to_add).expect("parsing nodes_vec failed");

    let ic_admin_args = [
        vec![
            "propose-to-add-nodes-to-subnet".to_string(),
            "--subnet".to_string(),
            subnet.id,
            "--proposer".to_string(),
            env_cfg("neuron_id"),
            "--proposal-title".to_string(),
            proposal_title.to_string(),
            "--summary".to_string(),
            proposal_summary.to_string(),
        ],
        nodes_vec.as_vec_string(),
    ]
    .concat();

    utils::ic_admin_run(&ic_admin_args, real_run)
}

pub fn propose_remove_nodes_from_subnet(
    nodes_to_remove: &str,
    real_run: bool,
    proposal_title: &str,
    proposal_summary: &str,
) -> Result<String, anyhow::Error> {
    let nodes_vec = NodesVec::from_str(nodes_to_remove).expect("parsing nodes_vec failed");

    let ic_admin_args = [
        vec![
            "propose-to-remove-nodes-from-subnet".to_string(),
            "--proposer".to_string(),
            env_cfg("neuron_id"),
            "--proposal-title".to_string(),
            proposal_title.to_string(),
            "--summary".to_string(),
            proposal_summary.to_string(),
        ],
        nodes_vec.as_vec_string(),
    ]
    .concat();

    utils::ic_admin_run(&ic_admin_args, real_run)
}

fn proposal_generate_summary(
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

fn proposal_generate_title(
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
        "(already done)"
    } else {
        "(in this proposal)"
    };
    let step_2_status = if nodes_to_add_proposal_id > 0 {
        "(in this proposal)"
    } else {
        "(in an upcoming proposal)"
    };

    format!(
        "Replace {} in subnet {}: add nodes {} {}; remove nodes {} {}",
        plural,
        subnet.id_short,
        nodes_add_vec.as_string_short(),
        step_1_status,
        nodes_remove_vec.as_string_short(),
        step_2_status,
    )
}

#[cfg(test)]
mod tests;
