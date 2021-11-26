use crate::schema::subnet_update_nodes;
use crate::schema::subnet_update_nodes::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use log::{debug, info};

// FIXME: proposal_add and proposal_remove should be factored out into a
// separate table.
#[derive(Queryable, Identifiable, AsChangeset, Debug)]
#[primary_key(id)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetUpdateNodes {
    pub id: i32,
    pub subnet: String,
    pub in_progress: bool,
    pub nodes_to_add: Option<String>,
    pub nodes_to_remove: Option<String>,
    pub proposal_add_id: Option<i64>,
    pub proposal_remove_id: Option<i64>,
}

#[derive(Insertable, Debug)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetNodesToAdd {
    pub subnet: String,
    pub nodes_to_add: String,
    pub in_progress: bool,
}

#[derive(Insertable, Debug)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetNodesToRemove {
    pub subnet: String,
    pub nodes_to_remove: String,
    pub in_progress: bool,
}

pub fn subnet_rows_get(connection: &SqliteConnection, subnet_id: &str) -> Vec<StateSubnetUpdateNodes> {
    match subnet_update_nodes
        .filter(subnet.eq(subnet_id))
        .load::<StateSubnetUpdateNodes>(connection.to_owned())
    {
        Ok(result) => result,
        Err(e) => panic!("Error executing filter query for subnet_id: {}. {}", subnet_id, e),
    }
}

pub fn subnet_rows_in_progress_get(connection: &SqliteConnection, subnet_id: &str) -> Vec<StateSubnetUpdateNodes> {
    let query = subnet_update_nodes.filter(subnet.eq(subnet_id).and(in_progress.eq(true)));
    match query.load::<StateSubnetUpdateNodes>(connection.to_owned()) {
        Ok(result) => {
            info!("result for subnet_id {}: {:?}", subnet_id, result);
            println!("{}", diesel::debug_query::<Sqlite, _>(&query));
            result
        }
        Err(e) => panic!("Error executing filter query for subnet_id: {}. {}", subnet_id, e),
    }
}

pub fn subnet_row_mark_completed(connection: &SqliteConnection, row: &StateSubnetUpdateNodes) {
    diesel::update(row)
        .set(in_progress.eq(false))
        .execute(connection)
        .expect("update failed");
}

pub fn subnet_nodes_to_add_set(
    connection: &SqliteConnection,
    subnet_id: &str,
    node_ids_to_add: &str,
) -> Result<(), anyhow::Error> {
    let subnet_rows = subnet_rows_in_progress_get(connection, subnet_id);
    for row in &subnet_rows {
        match &row.nodes_to_add {
            Some(row_nodes_to_add) => {
                if row_nodes_to_add != node_ids_to_add {
                    return Err(anyhow!(
                        "Subnet {} already has different nodes proposed to add: {} != {}",
                        subnet_id,
                        row_nodes_to_add,
                        node_ids_to_add
                    ));
                }
            }
            None => {
                info!(
                    "DB: updating existing row {}; setting nodes_to_add {}",
                    row.id, node_ids_to_add
                );

                diesel::update(row)
                    .set(nodes_to_add.eq(node_ids_to_add))
                    .execute(connection)
                    .expect("update failed");

                return Ok(());
            }
        }
    }

    let new_row = StateSubnetNodesToAdd {
        subnet: subnet_id.to_string(),
        nodes_to_add: node_ids_to_add.to_string(),
        in_progress: true,
    };
    info!("DB: inserting new row with nodes_to_add {:?}", new_row);
    diesel::insert_into(subnet_update_nodes)
        .values(&new_row)
        .execute(connection)
        .expect("insert_into failed");
    Ok(())
}

pub fn subnet_nodes_to_remove_set(
    connection: &SqliteConnection,
    subnet_id: &str,
    node_ids_to_remove: &str,
) -> Result<(), anyhow::Error> {
    let subnet_rows = subnet_rows_in_progress_get(connection, subnet_id);
    for row in &subnet_rows {
        match &row.nodes_to_remove {
            Some(row_nodes_to_remove) => {
                if row_nodes_to_remove != node_ids_to_remove {
                    return Err(anyhow!(
                        "Subnet {} already has different nodes proposed to remove: {} != {}",
                        subnet_id,
                        row_nodes_to_remove,
                        node_ids_to_remove
                    ));
                }
            }
            None => {
                info!(
                    "DB: updating existing row {}; setting nodes_to_remove {}",
                    row.id, node_ids_to_remove
                );

                diesel::update(row)
                    .set(nodes_to_remove.eq(node_ids_to_remove))
                    .execute(connection)
                    .expect("update failed");

                return Ok(());
            }
        }
    }

    let new_row = StateSubnetNodesToRemove {
        subnet: subnet_id.to_string(),
        nodes_to_remove: node_ids_to_remove.to_string(),
        in_progress: true,
    };
    info!("DB: inserting new row with nodes_to_remove {:?}", new_row);
    diesel::insert_into(subnet_update_nodes)
        .values(&new_row)
        .execute(connection)
        .expect("insert_into failed");
    Ok(())
}

pub fn subnet_nodes_to_add_update_proposal_id(
    connection: &SqliteConnection,
    subnet_id: &str,
    nodes_ids: &str,
    proposal_id: u64,
) -> Result<(), anyhow::Error> {
    let rows_in_progress = subnet_rows_in_progress_get(connection, subnet_id);
    debug!(
        "subnet_nodes_to_add_update_proposal_id rows_in_progress {:?}",
        rows_in_progress
    );
    for row in &rows_in_progress {
        if let Some(row_nodes) = &row.nodes_to_add {
            if row_nodes == nodes_ids {
                diesel::update(row)
                    .set(proposal_add_id.eq(proposal_id as i64))
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
    proposal_id: u64,
) -> Result<(), anyhow::Error> {
    let rows_in_progress = subnet_rows_in_progress_get(connection, subnet_id);
    debug!(
        "subnet_nodes_to_remove_update_proposal_id rows_in_progress {:?}",
        rows_in_progress
    );
    for row in &rows_in_progress {
        if let Some(row_nodes) = &row.nodes_to_remove {
            if row_nodes == nodes_ids {
                diesel::update(row)
                    .set(proposal_remove_id.eq(proposal_id as i64))
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
