use axum::{Json, extract::State, http::StatusCode};

use crate::handlers::Server;

pub(crate) async fn get_rate(State(state): State<Server>) -> Result<Json<u64>, (StatusCode, String)> {
    Ok(Json(state.get_rate().await))
}
