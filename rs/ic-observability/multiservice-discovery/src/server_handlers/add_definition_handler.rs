use slog::Logger;

use std::path::PathBuf;
use std::time::Duration;

use super::{bad_request, ok, WebResult};
use warp::Reply;

use crate::definition::{DefinitionsSupervisor, StartMode};
use crate::server_handlers::dto::DefinitionDto;

#[derive(Clone)]
pub(crate) struct AddDefinitionBinding {
    pub supervisor: DefinitionsSupervisor,
    pub log: Logger,
    pub registry_path: PathBuf,
    pub poll_interval: Duration,
    pub registry_query_timeout: Duration,
}

pub(crate) async fn add_definition(definition: DefinitionDto, binding: AddDefinitionBinding) -> WebResult<impl Reply> {
    let log = binding.log.clone();
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
        Err(e) => return bad_request(log, rej, e),
    };
    match binding
        .supervisor
        .start(vec![new_definition], StartMode::AddToDefinitions)
        .await
    {
        Ok(()) => ok(log, format!("Definition {} added successfully", dname)),
        Err(e) => bad_request(log, rej, e.errors.into_iter().next().unwrap()),
    }
}
