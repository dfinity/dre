use axum::{extract::State, http::StatusCode, Json};

use crate::handlers::Server;

pub async fn put_rate(State(state): State<Server>, Json(rate): Json<u64>) -> Result<Json<u64>, (StatusCode, String)> {
    Ok(Json(state.update_rate(rate).await))
}
