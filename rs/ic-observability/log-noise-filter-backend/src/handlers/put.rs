use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, Json};

use super::Server;

pub async fn update(State(state): State<Server>, Json(criteria): Json<Vec<String>>) -> Result<Json<BTreeMap<u32, String>>, (StatusCode, String)> {
    state.update_criteria(criteria).await;

    Ok(Json(state.get_criteria_mapped().await))
}
