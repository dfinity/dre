use crate::cli_types::{NodesVec, SubcmdSubnetUpdateNodes, Subnet};
use anyhow::anyhow;
use colored::Colorize;
use diesel::prelude::SqliteConnection;
use log::{debug, info, warn};
use regex::Regex;

use crate::models;
use crate::utils::{self, env_cfg};
use std::str::FromStr;

fn proposal_add_update_completed(
    db_connection: &SqliteConnection,
    row: &models::StateSubnetUpdateNodes,
) -> Result<Option<utils::ProposalStatus>, anyhow::Error> {
    match row.proposal_add_id {
        Some(proposal_id) => {
            let proposal = utils::get_proposal_status(proposal_id)?;
            if proposal.executed_timestamp_seconds > 0 {
                info!(
                    "Proposal {} for adding nodes has a completed status on the NNS.",
                    proposal_id
                );
                models::subnet_nodes_add_set_proposal_executed(
                    db_connection,
                    row.id,
                    proposal.executed_timestamp_seconds,
                )?;
            }
            if proposal.failed_timestamp_seconds > 0 {
                warn!(
                    "Proposal {} for adding nodes failed execution. Reason: {}",
                    proposal_id, proposal.failure_reason
                );
                models::subnet_nodes_add_set_proposal_failure(
                    db_connection,
                    row.id,
                    proposal.failed_timestamp_seconds,
                    &proposal.failure_reason,
                )?;
            }
            Ok(Some(proposal))
        }
        None => Ok(None),
    }
}

fn proposal_remove_update_completed(
    db_connection: &SqliteConnection,
    row: &models::StateSubnetUpdateNodes,
) -> Result<Option<utils::ProposalStatus>, anyhow::Error> {
    match row.proposal_remove_id {
        Some(proposal_id) => {
            let proposal = utils::get_proposal_status(proposal_id)?;
            if proposal.executed_timestamp_seconds > 0 {
                info!(
                    "Proposal {} for removing nodes has a completed status on the NNS.",
                    proposal_id
                );
                models::subnet_nodes_remove_set_proposal_executed(
                    db_connection,
                    row.id,
                    proposal.executed_timestamp_seconds,
                )?;
            }
            if proposal.failed_timestamp_seconds > 0 {
                warn!(
                    "Proposal {} for removing nodes failed execution. Reason: {}",
                    proposal_id, proposal.failure_reason
                );
                models::subnet_nodes_remove_set_proposal_failure(
                    db_connection,
                    row.id,
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
    let db_subnet_nodes_to_add = models::subnet_nodes_to_add_get(db_connection, subnet_id);

    let mut pending = false;
    for row in &db_subnet_nodes_to_add {
        match (&row.nodes_to_add, &row.nodes_to_remove) {
            (Some(nodes_to_add), Some(nodes_to_remove)) => {
                if row.proposal_add_executed_timestamp > 0 || row.proposal_add_failed_timestamp > 0 {
                    if row.proposal_remove_executed_timestamp > 0 || row.proposal_remove_failed_timestamp > 0 {
                        continue;
                    } else {
                        proposal_remove_update_completed(db_connection, &row)?;
                    }
                }
                match row.proposal_add_id {
                    None => {
                        info!("No proposal yet for adding nodes {}", nodes_to_add);
                        // No proposal submitted yet, let's do that now
                        let summary_long = proposal_summary_generate(subnet_id, nodes_to_add, nodes_to_remove, 0);
                        let summary_short =
                            proposal_summary_generate_one_line(subnet_id, nodes_to_add, nodes_to_remove, 0);
                        let proposal_summary_external_url =
                            utils::nns_proposals_repo_create_new_subnet_management(&summary_long, &summary_short)?;

                        let ic_admin_stdout = propose_add_nodes_to_subnet(
                            subnet_id,
                            nodes_to_add,
                            true,
                            &proposal_summary_external_url,
                            &summary_short,
                        )?;
                        let proposal_id = parse_proposal_id_from_ic_admin_stdout(ic_admin_stdout.as_str())?;
                        models::subnet_nodes_to_add_update_proposal_id(
                            db_connection,
                            &subnet_id.to_string(),
                            &nodes_to_add.to_string(),
                            proposal_id,
                        )?;
                    }
                    Some(add_proposal_id) => {
                        // Proposal already submitted
                        if row.proposal_add_executed_timestamp > 0 || row.proposal_add_failed_timestamp > 0 {
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
                            let proposal = proposal_add_update_completed(db_connection, row)?;
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
    let summary_long = proposal_summary_generate(subnet_id, nodes_to_add, nodes_to_remove, add_proposal_id);
    let summary_short = proposal_summary_generate_one_line(subnet_id, nodes_to_add, nodes_to_remove, add_proposal_id);
    let proposal_summary_external_url =
        utils::nns_proposals_repo_create_new_subnet_management(&summary_long, &summary_short)?;

    let ic_admin_stdout =
        propose_remove_nodes_from_subnet(nodes_to_remove, true, &proposal_summary_external_url, &summary_short)?;
    let proposal_id = parse_proposal_id_from_ic_admin_stdout(ic_admin_stdout.as_str())?;
    models::subnet_nodes_to_remove_update_proposal_id(
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
            for row in models::subnet_records_get(db_connection, &nodes.subnet) {
                if row.nodes_to_add == nodes.nodes_to_add && row.nodes_to_remove == nodes.nodes_to_remove {
                    let proposal_add = proposal_add_update_completed(db_connection, &row)?;
                    let proposal_remove = proposal_remove_update_completed(db_connection, &row)?;
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

            let summary_short = proposal_summary_generate_one_line(&nodes.subnet, nodes_to_add, nodes_to_remove, 0);
            propose_add_nodes_to_subnet(&nodes.subnet, nodes_to_add, false, "<tbd>", &summary_short)?;

            let summary_short = proposal_summary_generate_one_line(&nodes.subnet, nodes_to_add, nodes_to_remove, 1);
            propose_remove_nodes_from_subnet(nodes_to_remove, false, "<tbd>", &summary_short)?;

            // Success, user confirmed both operations
            models::subnet_nodes_to_replace_set(db_connection, &nodes.subnet, nodes_to_add, nodes_to_remove)?;

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
    proposal_url: &str,
    summary_short: &str,
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
            "--proposal-url".to_string(),
            proposal_url.to_string(),
            "--summary".to_string(),
            summary_short.to_string(),
        ],
        nodes_vec.as_vec_string(),
    ]
    .concat();

    utils::ic_admin_run(&ic_admin_args, real_run)
}

pub fn propose_remove_nodes_from_subnet(
    nodes_to_remove: &str,
    real_run: bool,
    proposal_url: &str,
    summary_short: &str,
) -> Result<String, anyhow::Error> {
    let nodes_vec = NodesVec::from_str(nodes_to_remove).expect("parsing nodes_vec failed");

    let ic_admin_args = [
        vec![
            "propose-to-remove-nodes-from-subnet".to_string(),
            "--proposer".to_string(),
            env_cfg("neuron_id"),
            "--proposal-url".to_string(),
            proposal_url.to_string(),
            "--summary".to_string(),
            summary_short.to_string(),
        ],
        nodes_vec.as_vec_string(),
    ]
    .concat();

    utils::ic_admin_run(&ic_admin_args, real_run)
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
