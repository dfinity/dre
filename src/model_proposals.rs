use crate::schema::proposals;
use crate::schema::proposals::dsl::*;
use diesel::prelude::*;
use log::{info, warn};

#[derive(Queryable, Identifiable, AsChangeset, Debug)]
#[primary_key(id)]
#[table_name = "proposals"]
pub struct Proposal {
    pub id: i64,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub submit_output: Option<String>,
    pub executed_timestamp: i64,
    pub failed_timestamp: i64,
    pub failed: Option<String>,
}

#[derive(Insertable, Debug)]
#[table_name = "proposals"]
pub struct ProposalAdd {
    pub id: i64,
    pub title: String,
    pub summary: String,
    pub submit_output: String,
}

pub fn proposal_get(connection: &SqliteConnection, proposal_id: i64) -> Proposal {
    match proposals::table.find(proposal_id).get_result::<Proposal>(connection) {
        Ok(result) => result,
        Err(e) => panic!("Error finding proposal_id: {}. {}", proposal_id, e),
    }
}

pub fn proposal_add(
    connection: &SqliteConnection,
    proposal_id: u64,
    proposal_title: &str,
    proposal_summary: &str,
    proposal_submit_output: &str,
) {
    let new_row = ProposalAdd {
        id: proposal_id as i64,
        title: proposal_title.to_string(),
        summary: proposal_summary.to_string(),
        submit_output: proposal_submit_output.to_string(),
    };
    info!("DB: inserting new Proposal row {:?}", new_row);
    diesel::insert_into(proposals)
        .values(&new_row)
        .execute(connection)
        .expect("insert_into failed");
}

pub fn is_proposal_executed(connection: &SqliteConnection, proposal_id: Option<i64>) -> bool {
    match proposal_id {
        Some(proposal_id) => {
            let proposal = proposal_get(connection, proposal_id);
            proposal.executed_timestamp > 0 || proposal.failed_timestamp > 0
        }
        None => false,
    }
}

pub fn proposal_set_executed(
    connection: &SqliteConnection,
    proposal_id: u64,
    timestamp: u64,
) -> Result<(), anyhow::Error> {
    info!("Proposal {}: marking as executed at {}", proposal_id, timestamp);
    diesel::update(proposals::table.find(proposal_id as i64))
        .set(executed_timestamp.eq(timestamp as i64))
        .execute(connection)?;
    Ok(())
}

pub fn proposal_set_failed(
    connection: &SqliteConnection,
    proposal_id: u64,
    failure_timestamp: u64,
    failure_reason: &str,
) -> Result<(), anyhow::Error> {
    warn!(
        "Proposal {}: marking as failed at {}. Reason: {}",
        proposal_id, failure_timestamp, failure_reason
    );
    diesel::update(proposals::table.find(proposal_id as i64))
        .set((failed_timestamp.eq(failure_timestamp as i64), failed.eq(failure_reason)))
        .execute(connection)?;
    Ok(())
}
