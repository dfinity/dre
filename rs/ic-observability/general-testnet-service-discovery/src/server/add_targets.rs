use axum::{extract::State, http::StatusCode, Json};
use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;

use crate::{storage::Storage, supervisor::TargetSupervisor};

use super::WebResult;

pub(crate) async fn add_targets(State(supervisor): State<TargetSupervisor>, Json(targets): Json<Vec<JournaldTarget>>) -> WebResult<()> {
    supervisor.upsert(targets).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}
