use crate::definition::DefinitionsSupervisor;
use crate::definition::StopDefinitionError;
use slog::Logger;
use warp::Reply;

use super::{forbidden, not_found, ok, WebResult};

#[derive(Clone)]
pub(super) struct DeleteDefinitionBinding {
    pub(crate) supervisor: DefinitionsSupervisor,
    pub(crate) log: Logger,
}

pub(super) async fn delete_definition(name: String, binding: DeleteDefinitionBinding) -> WebResult<impl Reply> {
    let rej = format!("Definition {} could not be deleted", name);
    match binding.supervisor.stop(vec![name.clone()]).await {
        Ok(_) => ok(binding.log, format!("Deleted definition {}", name.clone())),
        Err(e) => match e.errors.into_iter().next().unwrap() {
            StopDefinitionError::DoesNotExist(e) => {
                not_found(binding.log, "FUCK".to_string(), StopDefinitionError::DoesNotExist(e))
            }
            StopDefinitionError::DeletionDisallowed(e) => {
                forbidden(binding.log, rej, StopDefinitionError::DeletionDisallowed(e))
            }
        },
    }
}
