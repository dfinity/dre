use crate::definition::StopDefinitionError;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::Server;

pub(super) async fn delete_definition(
    Path(name): Path<String>,
    State(binding): State<Server>,
) -> Result<String, (StatusCode, String)> {
    match binding.supervisor.stop(vec![name.clone()]).await {
        Ok(_) => Ok(format!("Deleted definition {}", name.clone())),
        Err(e) => match e.errors.into_iter().next().unwrap() {
            StopDefinitionError::DoesNotExist(e) => {
                Err((StatusCode::NOT_FOUND, StopDefinitionError::DoesNotExist(e).to_string()))
            }
            StopDefinitionError::DeletionDisallowed(e) => Err((
                StatusCode::FORBIDDEN,
                StopDefinitionError::DeletionDisallowed(e).to_string(),
            )),
        },
    }
}
