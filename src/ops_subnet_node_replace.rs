use crate::cli::{NodesVec, Subnet};
use crate::ic_admin;
use anyhow::anyhow;
use colored::Colorize;
use decentralization::SubnetChangeResponse;
use diesel::prelude::SqliteConnection;
use ic_base_types::PrincipalId;
use log::{debug, info};
use regex::Regex;

use crate::model_proposals;
use crate::model_subnet_update_nodes;
use crate::utils;
use std::str::FromStr;

fn proposal_check_if_completed(
    db_connection: &SqliteConnection,
    proposal_id: Option<i64>,
) -> Result<Option<utils::ProposalStatus>, anyhow::Error> {
    match proposal_id {
        Some(proposal_id) => {
            let proposal = utils::get_proposal_status(proposal_id as u64)?;
            if proposal.executed_timestamp_seconds > 0 {
                model_proposals::proposal_set_executed(
                    db_connection,
                    proposal_id as u64,
                    proposal.executed_timestamp_seconds,
                )?;
            }
            if proposal.failed_timestamp_seconds > 0 {
                model_proposals::proposal_set_failed(
                    db_connection,
                    proposal_id as u64,
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
    ica: &ic_admin::CliDeprecated,
    db_connection: &SqliteConnection,
    subnet_id: &str,
) -> Result<bool, anyhow::Error> {
    let rows = model_subnet_update_nodes::subnet_rows_in_progress_get(db_connection, subnet_id);

    let mut pending = false;
    for row in &rows {
        if let (Some(nodes_to_add), Some(nodes_to_remove)) = (&row.nodes_to_add, &row.nodes_to_remove) {
            if !model_proposals::is_proposal_executed(db_connection, row.proposal_add_id) {
                proposal_check_if_completed(db_connection, row.proposal_add_id)?;
            }
            if !model_proposals::is_proposal_executed(db_connection, row.proposal_remove_id) {
                proposal_check_if_completed(db_connection, row.proposal_remove_id)?;
            }
            if model_proposals::is_proposal_executed(db_connection, row.proposal_add_id)
                && model_proposals::is_proposal_executed(db_connection, row.proposal_remove_id)
            {
                model_subnet_update_nodes::subnet_row_mark_completed(db_connection, row);
                continue;
            }
            match row.proposal_add_id {
                None => {
                    info!("No proposal yet for adding nodes {}", nodes_to_add);
                    // No proposal submitted yet, let's do that now
                    let proposal_summary = proposal_generate_summary(subnet_id, nodes_to_add, nodes_to_remove, 0);
                    let proposal_title = proposal_generate_title(subnet_id, nodes_to_add, nodes_to_remove, 0);

                    let ic_admin_stdout = ica.propose_run(
                        ic_admin::ProposeCommand::AddNodesToSubnet {
                            subnet_id: PrincipalId::from_str(subnet_id).unwrap(),
                            nodes: nodes_to_add
                                .split(',')
                                .map(|n| PrincipalId::from_str(n).unwrap())
                                .collect::<Vec<_>>(),
                        },
                        ic_admin::ProposeOptions {
                            summary: proposal_summary.clone().into(),
                            title: proposal_title.clone().into(),
                        },
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
                    let add_proposal_id = add_proposal_id as u64;
                    // Proposal already submitted
                    if model_proposals::is_proposal_executed(db_connection, row.proposal_add_id) {
                        // Add proposal already marked finished execution
                        if row.proposal_remove_id.is_none() {
                            // remove proposal was not created yet
                            info!("Proposal for adding nodes {} was already marked finished, proceeding with removal of {}", nodes_to_add, nodes_to_remove);
                            let proposal = utils::get_proposal_status(add_proposal_id as u64)?;
                            if proposal.executed_timestamp_seconds > 0 {
                                proposal_delete_create(
                                    ica,
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
                                    ica,
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
        pending = true;
    }
    Ok(pending)
}

fn proposal_delete_create(
    ica: &ic_admin::CliDeprecated,
    db_connection: &SqliteConnection,
    add_proposal_id: u64,
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

    let ic_admin_stdout = ica.propose_run(
        ic_admin::ProposeCommand::RemoveNodesFromSubnet {
            nodes: nodes_to_remove
                .split(',')
                .map(|n| PrincipalId::from_str(n).unwrap())
                .collect::<Vec<_>>(),
        },
        ic_admin::ProposeOptions {
            summary: proposal_summary.clone().into(),
            title: proposal_title.clone().into(),
        },
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
    model_subnet_update_nodes::subnet_nodes_to_remove_update_proposal_id(
        db_connection,
        &subnet_id.to_string(),
        &nodes_to_remove.to_string(),
        proposal_id,
    )?;
    Ok(())
}

pub fn subnet_nodes_replace(
    ica: &ic_admin::CliDeprecated,
    db_connection: &SqliteConnection,
    subnet: &str,
    nodes_to_add: Option<String>,
    nodes_to_remove: Option<String>,
) -> Result<String, anyhow::Error> {
    debug!(
        "subnet_update_nodes {} nodes: add {:?} remove {:?}",
        subnet, nodes_to_add, nodes_to_remove
    );

    check_and_submit_proposals_subnet_add_nodes(ica, db_connection, subnet)?;
    println!("Proposing to update nodes on subnet: {}", subnet.blue());

    match (nodes_to_add, nodes_to_remove) {
        (Some(nodes_to_add), Some(nodes_to_remove)) => {
            if let Some(row) = model_subnet_update_nodes::subnet_rows_get(db_connection, subnet)
                .into_iter()
                .next()
            {
                if row.nodes_to_add == nodes_to_add.into() && row.nodes_to_remove == nodes_to_remove.into() {
                    let proposal_add = proposal_check_if_completed(db_connection, row.proposal_add_id)?;
                    let proposal_remove = proposal_check_if_completed(db_connection, row.proposal_remove_id)?;
                    if let (Some(proposal_add), Some(proposal_remove)) = (&proposal_add, &proposal_remove) {
                        let add_finished =
                            proposal_add.executed_timestamp_seconds > 0 || proposal_add.failed_timestamp_seconds > 0;
                        let remove_finished = proposal_remove.executed_timestamp_seconds > 0
                            || proposal_remove.failed_timestamp_seconds > 0;
                        if add_finished && remove_finished {
                            info!("Both add and remove finished on subnet {}", subnet);
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

            let nodes_add_vec = NodesVec::from_str(&nodes_to_add).expect("parsing nodes_vec failed");
            let nodes_rm_vec = NodesVec::from_str(&nodes_to_remove).expect("parsing nodes_vec failed");
            println!(
                "{} {}",
                "Nodes to add:".green(),
                nodes_add_vec.as_string_short().green()
            );
            println!("{} {}", "Nodes to remove:".red(), nodes_rm_vec.as_string_short().red());

            let proposal_title = proposal_generate_title(subnet, &nodes_to_add, &nodes_to_remove, 0);
            let proposal_summary = proposal_generate_summary(subnet, &nodes_to_add, &nodes_to_remove, 0);
            ica.dry_run().propose_run(
                ic_admin::ProposeCommand::AddNodesToSubnet {
                    subnet_id: PrincipalId::from_str(subnet).unwrap(),
                    nodes: nodes_to_add
                        .split(',')
                        .map(|n| PrincipalId::from_str(n).unwrap())
                        .collect::<Vec<_>>(),
                },
                ic_admin::ProposeOptions {
                    summary: proposal_summary.into(),
                    title: proposal_title.into(),
                },
            )?;

            let proposal_title = proposal_generate_title(subnet, &nodes_to_add, &nodes_to_remove, 1);
            let proposal_summary = proposal_generate_summary(subnet, &nodes_to_add, &nodes_to_remove, 1);
            ica.dry_run().propose_run(
                ic_admin::ProposeCommand::RemoveNodesFromSubnet {
                    nodes: nodes_to_remove
                        .split(',')
                        .map(|n| PrincipalId::from_str(n).unwrap())
                        .collect::<Vec<_>>(),
                },
                ic_admin::ProposeOptions {
                    summary: proposal_summary.into(),
                    title: proposal_title.into(),
                },
            )?;

            // Success, user confirmed both operations
            model_subnet_update_nodes::subnet_nodes_to_replace_set(
                db_connection,
                subnet,
                &nodes_to_add,
                &nodes_to_remove,
            )?;

            Ok("All done".to_string())
        }
        _ => Err(anyhow!("Please provide both nodes to add and remove")),
    }
}

fn parse_proposal_id_from_ic_admin_stdout(text: &str) -> Result<u64, anyhow::Error> {
    let re = Regex::new(r"(?m)^proposal (\d+)$").unwrap();
    let mut captures = re.captures_iter(text);
    if let Some(cap) = captures.next() {
        let proposal_id = cap[1].parse::<u64>().unwrap();
        Ok(proposal_id)
    } else {
        Err(anyhow!(
            "The proposal number could not be extracted from the text:\n{}",
            text,
        ))
    }
}

fn proposal_generate_summary(
    subnet_id: &str,
    nodes_to_add: &str,
    nodes_to_remove: &str,
    nodes_to_add_proposal_id: u64,
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
        nodes_add_vec.as_string(),
        step_2_status,
        nodes_remove_vec.as_string(),
    )
}

fn proposal_generate_title(
    subnet_id: &str,
    nodes_to_add: &str,
    nodes_to_remove: &str,
    nodes_to_add_proposal_id: u64,
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

pub fn replace_proposal_options(
    change: &SubnetChangeResponse,
    first_proposal_id: Option<u64>,
) -> anyhow::Result<ic_admin::ProposeOptions> {
    let subnet_id = change
        .subnet_id
        .ok_or_else(|| anyhow::anyhow!("subnet_id is required"))?
        .to_string();

    let concat_principals =
        |principals: &[PrincipalId]| principals.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");

    let replace_target = if change.added.len() > 1 || change.removed.len() > 1 {
        "nodes"
    } else {
        "a node"
    };
    let subnet_id_short = subnet_id.split('-').next().unwrap();

    Ok(ic_admin::ProposeOptions {
        title: format!("Replace {replace_target} in subnet {subnet_id_short}",).into(),
        summary: format!(
            r#"# Replace {replace_target} in subnet {subnet_id_short}.

- Step 1 ({step_1_details}): Add nodes [{add_nodes}]
- Step 2 ({step_2_details}): Remove nodes [{remove_nodes}]
"#,
            step_1_details = first_proposal_id
                .map(|id| format!("proposal [{id}](https://dashboard.internetcomputer.org/proposal/{id})"))
                .unwrap_or_else(|| "this proposal".to_string()),
            add_nodes = concat_principals(&change.added),
            step_2_details = if first_proposal_id.is_some() {
                "this proposal"
            } else {
                "upcoming proposal"
            },
            remove_nodes = concat_principals(&change.removed),
        )
        .into(),
    })
}
