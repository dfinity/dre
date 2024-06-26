use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, Json};

use super::Server;

pub async fn delete_criteria(
    State(state): State<Server>,
    Json(criteria): Json<Vec<u32>>,
) -> Result<Json<BTreeMap<u32, String>>, (StatusCode, String)> {
    match state.delete_criteria(criteria).await {
        Ok(()) => Ok(Json(state.get_criteria_mapped().await)),
        Err(missing) => Err((StatusCode::NOT_FOUND, format!("Missing indexes: {:?}", missing))),
    }
}
