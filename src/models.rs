use crate::schema::subnet_update_nodes;
use crate::schema::subnet_update_nodes::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use log::info;

// FIXME: proposal_add and proposal_remove should be factored out into a
// separate table.
#[derive(Queryable, Identifiable, AsChangeset, Debug)]
#[primary_key(id)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetUpdateNodes {
    pub id: i32,
    pub subnet: String,
    pub nodes_to_add: Option<String>,
    pub nodes_to_remove: Option<String>,
    pub proposal_add_id: Option<i32>,
    pub proposal_add_executed_timestamp: i64,
    pub proposal_add_failed: Option<String>,
    pub proposal_remove_id: Option<i32>,
    pub proposal_remove_executed_timestamp: i64,
    pub proposal_remove_failed: Option<String>,
}

#[derive(Insertable, Debug)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetNodesToAdd {
    pub subnet: String,
    pub nodes_to_add: String,
}

#[derive(Insertable, Debug)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetNodesToRemove {
    pub subnet: String,
    pub nodes_to_remove: String,
}

pub fn subnet_records_get(connection: &SqliteConnection, subnet_id: &str) -> Vec<StateSubnetUpdateNodes> {
    match subnet_update_nodes
        .filter(subnet.eq(subnet_id))
        .load::<StateSubnetUpdateNodes>(connection.to_owned())
    {
        Ok(result) => result,
        Err(e) => panic!("Error executing filter query for subnet_id: {}. {}", subnet_id, e),
    }
}

pub fn subnet_nodes_to_add_get(connection: &SqliteConnection, subnet_id: &str) -> Vec<StateSubnetUpdateNodes> {
    match subnet_update_nodes
        .filter(
            subnet.eq(subnet_id).and(
                proposal_add_executed_timestamp
                    .eq(0)
                    .or(proposal_add_failed.is_not_null()),
            ),
        )
        .load::<StateSubnetUpdateNodes>(connection.to_owned())
    {
        Ok(result) => result,
        Err(e) => panic!("Error executing filter query for subnet_id: {}. {}", subnet_id, e),
    }
}

pub fn subnet_nodes_to_remove_get(connection: &SqliteConnection, subnet_id: &str) -> Vec<StateSubnetUpdateNodes> {
    match subnet_update_nodes
        .filter(
            subnet.eq(subnet_id).and(
                proposal_remove_executed_timestamp
                    .eq(0)
                    .or(proposal_remove_failed.is_not_null()),
            ),
        )
        .load::<StateSubnetUpdateNodes>(connection.to_owned())
    {
        Ok(result) => result,
        Err(e) => panic!("Error executing filter query for subnet_id: {}. {}", subnet_id, e),
    }
}

pub fn subnet_nodes_add_set_proposal_executed(
    connection: &SqliteConnection,
    row_id: i32,
    timestamp: i64,
) -> Result<(), anyhow::Error> {
    diesel::update(subnet_update_nodes.find(row_id))
        .set(proposal_add_executed_timestamp.eq(timestamp))
        .execute(connection)?;
    Ok(())
}

pub fn subnet_nodes_add_set_proposal_failure(
    connection: &SqliteConnection,
    row_id: i32,
    failure_reason: &str,
) -> Result<(), anyhow::Error> {
    diesel::update(subnet_update_nodes.find(row_id))
        .set(proposal_add_failed.eq(failure_reason))
        .execute(connection)?;
    Ok(())
}

#[allow(dead_code)]
pub fn subnet_nodes_remove_set_proposal_executed(
    connection: &SqliteConnection,
    row_id: i32,
    timestamp: i64,
) -> Result<(), anyhow::Error> {
    diesel::update(subnet_update_nodes.find(row_id))
        .set(proposal_remove_executed_timestamp.eq(timestamp))
        .execute(connection)?;
    Ok(())
}

pub fn subnet_nodes_to_add_set(
    connection: &SqliteConnection,
    subnet_id: &str,
    nodes_ids_to_add: &str,
) -> Result<(), anyhow::Error> {
    let existing_entries_nodes_to_add = subnet_nodes_to_add_get(connection, subnet_id);
    for row in &existing_entries_nodes_to_add {
        if let Some(row_nodes) = &row.nodes_to_add {
            if row_nodes != nodes_ids_to_add {
                return Err(anyhow!(
                    "Subnet {} already has different nodes proposed to add: {}",
                    subnet_id,
                    row_nodes,
                ));
            }
        }
    }

    // Update the existing entry
    if !existing_entries_nodes_to_add.is_empty() {
        let row_id = existing_entries_nodes_to_add[0].id;
        info!(
            "DB: updating existing row {} with nodes_to_add {}",
            row_id, nodes_ids_to_add
        );
        diesel::update(subnet_update_nodes.filter(id.eq(row_id)))
            .set(nodes_to_add.eq(nodes_ids_to_add))
            .execute(connection)
            .expect("update failed");
    } else {
        let new_row = StateSubnetNodesToAdd {
            subnet: subnet_id.to_string(),
            nodes_to_add: nodes_ids_to_add.to_string(),
        };
        info!("DB: inserting new row with nodes_to_add {:?}", new_row);
        diesel::insert_into(subnet_update_nodes)
            .values(&new_row)
            .execute(connection)
            .expect("insert_into failed");
    }
    Ok(())
}

pub fn subnet_nodes_to_remove_set(
    connection: &SqliteConnection,
    subnet_id: &str,
    nodes_ids_to_remove: &str,
) -> Result<(), anyhow::Error> {
    let existing_entries_nodes_to_remove = subnet_nodes_to_remove_get(connection, subnet_id);
    for row in &existing_entries_nodes_to_remove {
        if let Some(row_nodes) = &row.nodes_to_remove {
            if row_nodes != nodes_ids_to_remove {
                return Err(anyhow!(
                    "Subnet {} already has different nodes proposed to remove: {}",
                    subnet_id,
                    row_nodes,
                ));
            }
        }
    }

    // Update the existing entry
    if !existing_entries_nodes_to_remove.is_empty() {
        let row_id = existing_entries_nodes_to_remove[0].id;
        info!(
            "DB: updating existing row {} with nodes_to_remove {}",
            row_id, nodes_ids_to_remove
        );
        diesel::update(subnet_update_nodes.filter(id.eq(row_id)))
            .set(nodes_to_remove.eq(nodes_ids_to_remove))
            .execute(connection)
            .expect("update failed");
    } else {
        let new_row = StateSubnetNodesToRemove {
            subnet: subnet_id.to_string(),
            nodes_to_remove: nodes_ids_to_remove.to_string(),
        };
        info!("DB: inserting new row with nodes_to_remove {:?}", new_row);
        diesel::insert_into(subnet_update_nodes)
            .values(&new_row)
            .execute(connection)
            .expect("insert_into failed");
    }
    Ok(())
}

pub fn subnet_nodes_to_add_update_proposal_id(
    connection: &SqliteConnection,
    subnet_id: &str,
    nodes_ids: &str,
    proposal_id: i32,
) -> Result<(), anyhow::Error> {
    let existing_entries_nodes_to_add = subnet_nodes_to_add_get(connection, subnet_id);
    for row in &existing_entries_nodes_to_add {
        if let Some(row_nodes) = &row.nodes_to_add {
            if row_nodes == nodes_ids {
                diesel::update(row)
                    .set(proposal_add_id.eq(proposal_id))
                    .execute(connection)?;
                return Ok(());
            }
        }
    }
    Err(anyhow!(
        "subnet_nodes_to_add_update_proposal_id {}: no entry for node add found: {}",
        subnet_id,
        nodes_ids,
    ))
}

pub fn subnet_nodes_to_remove_update_proposal_id(
    connection: &SqliteConnection,
    subnet_id: &str,
    nodes_ids: &str,
    proposal_id: i32,
) -> Result<(), anyhow::Error> {
    let existing_entries_nodes_to_remove = subnet_nodes_to_remove_get(connection, subnet_id);
    for row in &existing_entries_nodes_to_remove {
        if let Some(row_nodes) = &row.nodes_to_remove {
            if row_nodes == nodes_ids {
                diesel::update(row)
                    .set(proposal_remove_id.eq(proposal_id))
                    .execute(connection)?;
                return Ok(());
            }
        }
    }
    Err(anyhow!(
        "subnet_nodes_to_remove_update_proposal_id {}: no entry for node remove found: {}",
        subnet_id,
        nodes_ids,
    ))
}

pub fn subnet_nodes_to_replace_set(
    connection: &SqliteConnection,
    subnet_id: &str,
    nodes_ids_to_add: &str,
    nodes_ids_to_remove: &str,
) -> Result<(), anyhow::Error> {
    subnet_nodes_to_add_set(connection, subnet_id, nodes_ids_to_add)?;
    subnet_nodes_to_remove_set(connection, subnet_id, nodes_ids_to_remove)
}
