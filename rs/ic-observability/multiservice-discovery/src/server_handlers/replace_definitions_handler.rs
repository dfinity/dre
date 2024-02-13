use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use futures::future::join_all;

use crate::definition::{Definition, StartMode};
use crate::server_handlers::dto::{BadDtoError, DefinitionDto};

use super::{bad_request, ok, Server, WebResult};

pub(super) async fn replace_definitions(
    State(binding): State<Server>,
    Json(definitions): Json<Vec<DefinitionDto>>,
) -> WebResult<String> {
    // Cache old names if we need to remove them from metrics
    let dnames = definitions
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<String>>()
        .join(", ");

    let defsresults_futures: Vec<_> = definitions
        .into_iter()
        .map(|tentative_definition| async {
            tentative_definition
                .try_into_definition(
                    binding.log.clone(),
                    binding.registry_path.clone(),
                    binding.poll_interval,
                    binding.registry_query_timeout,
                )
                .await
        })
        .collect();
    let defsresults: Vec<Result<Definition, BadDtoError>> = join_all(defsresults_futures).await;
    let (new_definitions, errors): (Vec<_>, Vec<_>) = defsresults.into_iter().partition(Result::is_ok);

    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();
    if !errors.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                ":\n * {}",
                errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n * ")
            ),
        ));
    }

    let new_definitions: Vec<_> = new_definitions.into_iter().map(Result::unwrap).collect();
    match binding
        .supervisor
        .start(
            new_definitions.clone(),
            StartMode::ReplaceExistingDefinitions,
            binding.metrics.running_definition_metrics.clone(),
        )
        .await
    {
        Ok(_) => ok(
            binding.log,
            format!("Added new definitions {} to existing ones", dnames),
        ),
        Err(e) => bad_request(binding.log, format!(":\n{}", e), e),
    }
}
