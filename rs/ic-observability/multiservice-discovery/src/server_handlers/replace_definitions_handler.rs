use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};

use slog::{error, info};

use warp::Reply;

use crate::definition::{wrap, Definition};
use crate::server_handlers::dto::{BadDtoError, DefinitionDto};
use crate::server_handlers::AddDefinitionBinding as ReplaceDefinitionsBinding;
use crate::server_handlers::WebResult;

#[derive(Debug)]
struct ReplaceDefinitionsError {
    errors: Vec<BadDtoError>,
}

impl Error for ReplaceDefinitionsError {}

impl Display for ReplaceDefinitionsError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        for e in self.errors.iter() {
            write!(f, "* {}", e)?
        }
        Ok(())
    }
}

async fn _replace_definitions(
    tentative_definitions: Vec<DefinitionDto>,
    binding: ReplaceDefinitionsBinding,
) -> Result<(), ReplaceDefinitionsError> {
    let mut existing_definitions = binding.definitions.lock().await;

    // Move all existing definitions to backed up lists.
    let mut backed_up_definitions: Vec<Definition> = vec![];
    for def in existing_definitions.drain(..) {
        info!(binding.log, "Moving definition {} from existing to backup", def.name);
        backed_up_definitions.push(def);
    }
    info!(binding.log, "Finished backing up existing definitions");

    // Add all-new definitions, checking them all and saving errors
    // as they happen.  Do not start their threads yet.
    let mut error = ReplaceDefinitionsError { errors: vec![] };
    for tentative_definition in tentative_definitions {
        let dname = tentative_definition.name.clone();
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
            Err(e) => {
                error.errors.push(e);
                continue;
            }
        };
        info!(binding.log, "Adding new definition {} to existing", dname);
        existing_definitions.push(new_definition);
    }

    // Was there an error?  Restore the definitions and handles to their
    // original structures.  From the point of view of the rest of the
    // program, nothing has changed here because this struct was locked.
    if !error.errors.is_empty() {
        for def in backed_up_definitions.drain(..) {
            info!(binding.log, "Restoring backed up definition {} to existing", def.name);
            existing_definitions.push(def);
        }
        info!(binding.log, "Finished restoring backed up definitions");
        return Err(error);
    }

    // Send stop signals to all old definitions...
    for old_definition in backed_up_definitions.iter() {
        info!(
            binding.log,
            "Sending termination signal to definition {}", old_definition.name
        );
        old_definition.stop_signal_sender.send(()).unwrap();
    }
    // ...and join their threads, emptying the handles vector...
    let mut existing_handles = binding.handles.lock().await;
    for old_handle in existing_handles.drain(..) {
        info!(binding.log, "Waiting for thread to finish...");
        if let Err(e) = old_handle.join() {
            error!(
                binding.log,
                "Could not join thread handle of definition being removed: {:?}", e
            );
        }
    }
    // ...then start and record the handles for the new definitions.
    for new_definition in existing_definitions.iter() {
        info!(binding.log, "Starting thread for definition: {:?}", new_definition.name);
        let joinhandle = std::thread::spawn(wrap(new_definition.clone(), binding.rt.clone()));
        existing_handles.push(joinhandle);
    }

    Ok(())
}

pub async fn replace_definitions(
    definitions: Vec<DefinitionDto>,
    binding: ReplaceDefinitionsBinding,
) -> WebResult<impl Reply> {
    let log = binding.log.clone();
    let dnames = definitions
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<String>>()
        .join(", ");
    match _replace_definitions(definitions, binding).await {
        Ok(_) => {
            info!(log, "Added new definitions {} to existing ones", dnames);
            Ok(warp::reply::with_status(
                format!("Definitions {} added successfully", dnames),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            error!(log, "Definitions could not be added:\n{}", e);
            Ok(warp::reply::with_status(
                format!("Definitions could not be replaced:\n{}", e),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
