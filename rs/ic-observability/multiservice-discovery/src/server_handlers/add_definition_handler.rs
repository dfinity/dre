use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::definition::StartMode;
use crate::server_handlers::dto::DefinitionDto;

use super::{bad_request, ok, Server};

pub(super) async fn add_definition(
    State(binding): State<Server>,
    Json(definition): Json<DefinitionDto>,
) -> Result<String, (StatusCode, String)> {
    let dname = definition.name.clone();
    let rej = format!("Definition {} could not be added", dname);
    let new_definition = match definition
        .try_into_definition(
            binding.log.clone(),
            binding.registry_path.clone(),
            binding.poll_interval,
            binding.registry_query_timeout,
        )
        .await
    {
        Ok(def) => def,
        Err(e) => return bad_request(binding.log, rej, e),
    };
    match binding
        .supervisor
        .start(vec![new_definition], StartMode::AddToDefinitions)
        .await
    {
        Ok(()) => {
            binding.metrics.definitions.add(1, &vec![]);
            ok(binding.log, format!("Definition {} added successfully", dname))
        }
        Err(e) => bad_request(binding.log, rej, e),
    }
}
