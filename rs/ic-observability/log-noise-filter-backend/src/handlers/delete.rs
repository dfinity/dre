use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, Json};
use slog::{info, warn};

use super::Server;

pub async fn delete_criteria(
    State(state): State<Server>,
    Json(criteria): Json<Vec<u32>>,
) -> Result<Json<BTreeMap<u32, String>>, (StatusCode, String)> {
    match state.delete_criteria(criteria.clone()).await {
        Ok(()) => {
            info!(state.logger, "Deleted criteria"; "indexes" => ?criteria);
            Ok(Json(state.get_criteria_mapped().await))
        }
        Err(missing) => {
            warn!(state.logger, "Failed to delete criteria"; "indexes" => ?missing);
            Err((StatusCode::NOT_FOUND, format!("Missing indexes: {:?}", missing)))
        }
    }
}
