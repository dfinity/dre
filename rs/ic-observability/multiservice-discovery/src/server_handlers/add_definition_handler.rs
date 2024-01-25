use slog::{error, info, Logger};

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::sync::Mutex;
use warp::Reply;

use crate::definition::{wrap, Definition};
use crate::server_handlers::dto::DefinitionDto;
use crate::server_handlers::WebResult;

use super::dto::BadDtoError;

pub(crate) struct AddDefinitionBinding {
    pub definitions: Arc<Mutex<Vec<Definition>>>,
    pub log: Logger,
    pub registry_path: PathBuf,
    pub poll_interval: Duration,
    pub registry_query_timeout: Duration,
    pub rt: tokio::runtime::Handle,
    pub handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

async fn _add_definition(
    tentative_definition: DefinitionDto,
    binding: AddDefinitionBinding,
) -> Result<(), BadDtoError> {
    let mut existing_definitions = binding.definitions.lock().await;

    let dname = tentative_definition.name.clone();
    if existing_definitions.iter().any(|d| d.name == tentative_definition.name) {
        return Err(BadDtoError::AlreadyExists(dname));
    }

    let new_definition = match tentative_definition
        .try_into_definition(
            binding.log.clone(),
            binding.registry_path.clone(),
            binding.poll_interval,
            binding.registry_query_timeout,
        )
        .await
    {
        Ok(def) => def,
        Err(e) => return Err(e),
    };
    info!(binding.log, "Adding new definition {} to existing", dname);
    existing_definitions.push(new_definition.clone());

    let mut existing_handles = binding.handles.lock().await;
    // ...then start and record the handles for the new definitions.
    info!(binding.log, "Starting thread for definition: {:?}", dname);
    let joinhandle = std::thread::spawn(wrap(new_definition, binding.rt));
    existing_handles.push(joinhandle);

    Ok(())
}

pub(crate) async fn add_definition(definition: DefinitionDto, binding: AddDefinitionBinding) -> WebResult<impl Reply> {
    let log = binding.log.clone();
    let dname = definition.name.clone();
    match _add_definition(definition, binding).await {
        Ok(_) => {
            info!(log, "Added new definition {} to existing ones", dname);
            Ok(warp::reply::with_status(
                format!("Definition {} added successfully", dname),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            error!(log, "Definition could not be added: {}", e);
            Ok(warp::reply::with_status(
                format!("Definition could not be added: {}", e),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
