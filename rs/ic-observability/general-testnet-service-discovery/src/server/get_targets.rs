use axum::{Json, extract::State, http::StatusCode};
use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;

use crate::{storage::Storage, supervisor::TargetSupervisor};

use super::WebResult;

pub(super) async fn get_targets(State(supervisor): State<TargetSupervisor>) -> WebResult<Json<Vec<JournaldTarget>>> {
    supervisor.get().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string())).map(Json)
}
