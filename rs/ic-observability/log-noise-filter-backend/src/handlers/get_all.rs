use std::{collections::BTreeMap, hash::Hash};

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use super::Server;

pub async fn get_all(State(server): State<Server>) -> Result<Json<WholeState>, (StatusCode, String)> {
    let state = WholeState {
        criteria: server.get_criteria_mapped().await,
        rate: server.get_rate().await,
    };

    Ok(Json(state))
}

#[derive(Serialize, Deserialize)]
pub struct WholeState {
    pub rate: u64,
    pub criteria: BTreeMap<u32, String>,
}

impl Hash for WholeState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.rate.hash(state);
        self.criteria.hash(state);
    }
}
