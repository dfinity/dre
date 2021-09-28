use r2d2::{PooledConnection, Pool};
use r2d2_sqlite::{SqliteConnectionManager};
use rusqlite::{params};
pub struct ReplacementStateWorker {
    db: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    not_added: Vec<(String, String)>,
}

impl ReplacementStateWorker {
    pub fn new(db: Pool<SqliteConnectionManager>) -> Self {
        db.get().expect("Unable to get pool connection").execute(
            "CREATE TABLE IF NOT EXISTS replacement_queue (waiting TEXT removed TEXT)", params![]
        )
        .expect("Unable to create needed database table");
        return ReplacementStateWorker {
            db,
            not_added: Vec::new()
        }
    }
    pub fn add_waited_replacement(&mut self, proposal: String, to_remove: String) {
        self.not_added.push((proposal, to_remove));
    }

    pub fn update_proposals(self) {
        for (proposal, to_remove) in self.not_added.iter() {
            self.db.execute(
                "INSERT into replacement_queue (a, b) VALUES (?, ?)", params![proposal, to_remove]
            );
        }
        let mut waiting = self.db.
    }

    async fn get_proposal_status(proposal_id: String) -> bool {
        /// Didn't quite get there in implementation
        /// The purpose of this function is to query the NNS to see if the proposal has been completed or not
        return true
    }
} 