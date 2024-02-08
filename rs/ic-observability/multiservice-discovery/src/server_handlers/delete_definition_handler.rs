use crate::definition::StopDefinitionError;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::{forbidden, not_found, Server};

pub(super) async fn delete_definition(
    Path(name): Path<String>,
    State(binding): State<Server>,
) -> Result<String, (StatusCode, String)> {
    match binding.supervisor.stop(vec![name.clone()]).await {
        Ok(_) => {
            binding
                .metrics
                .definitions
                .observe(binding.supervisor.definition_count().await as u64, &vec![]);
            Ok(format!("Deleted definition {}", name.clone()))
        }
        Err(e) => match e.errors.into_iter().next().unwrap() {
            StopDefinitionError::DoesNotExist(e) => {
                not_found(binding.log, format!("Definition with name '{}' doesn't exist", name), e)
            }
            StopDefinitionError::DeletionDisallowed(e) => {
                forbidden(binding.log, "That definition cannot be deleted".to_string(), e)
            }
        },
    }
}
