use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::{extract::State, Json};

use super::Server;

pub(crate) async fn get_criteria(State(state): State<Server>) -> Result<Json<BTreeMap<u32, String>>, (StatusCode, String)> {
    Ok(Json(state.get_criteria_mapped().await))
}
