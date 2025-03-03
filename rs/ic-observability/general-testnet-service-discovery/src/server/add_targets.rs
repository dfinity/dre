use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;

use crate::storage::Storage;

use super::WebResult;

pub(crate) async fn add_targets(State(storage): State<Arc<dyn Storage>>, Json(targets): Json<Vec<JournaldTarget>>) -> WebResult<()> {
    storage.upsert(targets).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}
