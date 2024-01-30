use crate::definition::DefinitionsSupervisor;
use crate::definition::StopDefinitionError;
use slog::Logger;
use warp::Reply;

use super::{bad_request, not_found, ok, WebResult};

#[derive(Clone)]
pub(crate) struct DeleteDefinitionBinding {
    pub supervisor: DefinitionsSupervisor,
    pub log: Logger,
}

pub async fn delete_definition(name: String, binding: DeleteDefinitionBinding) -> WebResult<impl Reply> {
    let rej = format!("Definition {} could not be deleted", name);
    if name == "ic" {
        return bad_request(
            binding.log,
            "Cannot delete ic definition".to_string(),
            "definition is not removable",
        );
    }

    match binding.supervisor.stop(vec![name.clone()]).await {
        Ok(_) => ok(binding.log, format!("Deleted definition {}", name.clone())),
        Err(e) => match e.errors.into_iter().next().unwrap() {
            StopDefinitionError::DoesNotExist(e) => not_found(binding.log, rej, e),
        },
    }
}
