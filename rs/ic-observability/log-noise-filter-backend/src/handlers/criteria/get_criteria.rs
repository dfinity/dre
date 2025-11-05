use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::{Json, extract::State};

use crate::handlers::Server;

pub(crate) async fn get_criteria(State(state): State<Server>) -> Result<Json<BTreeMap<u32, String>>, (StatusCode, String)> {
    Ok(Json(state.get_criteria_mapped().await))
}
