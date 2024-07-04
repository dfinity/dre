use axum::{extract::State, http::StatusCode, Json};

use super::{Server, WholeState};

pub async fn get_all(State(server): State<Server>) -> Result<Json<WholeState>, (StatusCode, String)> {
    let state = WholeState {
        criteria: server.get_criteria_mapped().await,
        rate: server.get_rate().await,
    };

    Ok(Json(state))
}
