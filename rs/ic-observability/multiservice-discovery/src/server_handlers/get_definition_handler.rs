use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::server_handlers::dto::DefinitionDto;

use super::Server;

pub(super) async fn get_definitions(State(supervisor): State<Server>) -> Result<Json<Vec<DefinitionDto>>, (StatusCode, String)> {
    let definitions = supervisor.supervisor.definitions.lock().await;

    let list = definitions
        .iter()
        .map(|(_, d)| {
            let x = &d.definition;
            x.into()
        })
        .collect::<Vec<DefinitionDto>>();
    Ok(Json(list))
}
