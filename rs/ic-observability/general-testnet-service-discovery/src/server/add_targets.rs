use axum::{Json, extract::State, http::StatusCode};
use multiservice_discovery_shared::contracts::journald_target::JournaldTarget;

use crate::{storage::Storage, supervisor::TargetSupervisor};

use super::WebResult;

pub(crate) async fn add_targets(State(supervisor): State<TargetSupervisor>, Json(target): Json<JournaldTarget>) -> WebResult<()> {
    supervisor.insert(target).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}
