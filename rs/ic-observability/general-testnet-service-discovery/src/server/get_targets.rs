use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;

use crate::storage::Storage;

use super::WebResult;

pub(super) async fn get_targets(State(storage): State<Arc<dyn Storage>>) -> WebResult<Json<Vec<JournaldTarget>>> {
    storage
        .get()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
        .map(|resp| Json(resp))
}
