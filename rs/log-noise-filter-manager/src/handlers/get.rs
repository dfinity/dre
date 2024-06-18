use axum::http::StatusCode;
use axum::{extract::State, Json};

use super::{Server, TopLevelVectorTransform, SEPARATOR};

pub(crate) async fn content(State(state): State<Server>) -> Result<Json<TopLevelVectorTransform>, (StatusCode, String)> {
    Ok(Json(state.read_file().await))
}

pub(crate) async fn only_routes(State(state): State<Server>) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let content = state.read_file().await;
    Ok(Json(
        content
            .transforms
            .noise_filter
            .route
            .noisy
            .split(SEPARATOR)
            .map(|t| t.to_string())
            .collect(),
    ))
}
