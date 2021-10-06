use crate::schema::subnet_update_nodes;
use crate::schema::subnet_update_nodes::dsl::*;
use anyhow::anyhow;
use diesel::prelude::*;
use log::info;

#[derive(Queryable, Identifiable, AsChangeset, Debug)]
#[primary_key(id)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetUpdateNodes {
    pub id: i32,
    pub subnet: String,
    pub nodes_to_add: Option<String>,
    pub nodes_to_remove: Option<String>,
    pub proposal_id_for_add: Option<i32>,
    pub proposal_executed_add: bool,
    pub proposal_id_for_remove: Option<i32>,
    pub proposal_executed_remove: bool,
}

#[derive(Insertable, Debug)]
#[table_name = "subnet_update_nodes"]
pub struct StateSubnetNodesToAdd {
    pub subnet: String,
    pub nodes_to_add: String,
}

pub fn subnet_nodes_to_add_get(
    connection: &SqliteConnection,
    subnet_id: &String,
) -> Vec<StateSubnetUpdateNodes> {
    match subnet_update_nodes
        .filter(subnet.eq(subnet_id).and(proposal_executed_add.ne(true)))
        .load::<StateSubnetUpdateNodes>(connection.clone())
    {
        Ok(result) => result,
        Err(e) => panic!(
            "Error executing filter query for subnet_id: {}. {}",
            subnet_id, e
        ),
    }
}

pub fn subnet_nodes_to_remove_get(
    connection: &SqliteConnection,
    subnet_id: &String,
) -> Vec<StateSubnetUpdateNodes> {
    match subnet_update_nodes
        .filter(subnet.eq(subnet_id).and(proposal_executed_remove.ne(true)))
        .load::<StateSubnetUpdateNodes>(connection.clone())
    {
        Ok(result) => result,
        Err(e) => panic!(
            "Error executing filter query for subnet_id: {}. {}",
            subnet_id, e
        ),
    }
}

pub fn subnet_nodes_to_add_set(
    connection: &SqliteConnection,
    subnet_id: &String,
    nodes_ids_to_add: &String,
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
    if existing_entries_nodes_to_add.len() > 0 {
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
            subnet: subnet_id.clone(),
            nodes_to_add: nodes_ids_to_add.clone(),
        };
        info!("DB: Adding new row with nodes_to_add {:?}", new_row);
        diesel::insert_into(subnet_update_nodes)
            .values(&new_row)
            .execute(connection)
            .expect("insert_into failed");
    }
    Ok(())
}

pub fn subnet_nodes_to_add_update_proposal_id(
    connection: &SqliteConnection,
    subnet_id: &String,
    nodes_ids: &String,
    proposal_id: i32,
) -> Result<(), anyhow::Error> {
    let existing_entries_nodes_to_add = subnet_nodes_to_add_get(connection, subnet_id);
    for row in &existing_entries_nodes_to_add {
        if let Some(row_nodes) = &row.nodes_to_add {
            if row_nodes == nodes_ids {
                diesel::update(row)
                    .set(proposal_id_for_add.eq(proposal_id))
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
    subnet_id: &String,
    nodes_ids: &String,
    proposal_id: i32,
) -> Result<(), anyhow::Error> {
    let existing_entries_nodes_to_remove = subnet_nodes_to_remove_get(connection, subnet_id);
    for row in &existing_entries_nodes_to_remove {
        if let Some(row_nodes) = &row.nodes_to_remove {
            if row_nodes == nodes_ids {
                diesel::update(row)
                    .set(proposal_id_for_remove.eq(proposal_id))
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
