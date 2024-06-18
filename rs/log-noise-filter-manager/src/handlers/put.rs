use axum::{extract::State, http::StatusCode, Json};

use super::{Server, TopLevelVectorTransform, SEPARATOR};

pub async fn update(State(state): State<Server>, Json(routes): Json<Vec<String>>) -> Result<Json<TopLevelVectorTransform>, (StatusCode, String)> {
    let mut content = state.read_file().await;
    if routes.is_empty() {
        content.transforms.noise_filter.route.noisy = "false".to_string();
    } else {
        content.transforms.noise_filter.route.noisy = routes.join(SEPARATOR)
    }

    state.write_structure(&content).await;
    Ok(Json(content))
}
