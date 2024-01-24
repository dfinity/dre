use base64::{engine::general_purpose as b64, Engine as _};
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};

use ic_crypto_utils_threshold_sig_der::parse_threshold_sig_key_from_der;
use service_discovery::registry_sync::nns_reachable;
use slog::{error, info};

use warp::Reply;

use crate::definition::{wrap, Definition};
use crate::server_handlers::add_definition_handler::AddDefinitionError as ReplaceDefinitionError;
use crate::server_handlers::dto::DefinitionDto;
use crate::server_handlers::AddDefinitionBinding as ReplaceDefinitionsBinding;
use crate::server_handlers::WebResult;

#[derive(Debug)]
struct ReplaceDefinitionsError {
    errors: Vec<ReplaceDefinitionError>,
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
    let mut existing_handles = binding.handles.lock().await;

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
        let public_key = match tentative_definition.public_key {
            Some(pk) => {
                let decoded = b64::STANDARD.decode(pk).unwrap();

                match parse_threshold_sig_key_from_der(&decoded) {
                    Ok(key) => Some(key),
                    Err(e) => {
                        error!(
                            binding.log,
                            "Submitted definition {} has invalid public key", tentative_definition.name
                        );
                        error
                            .errors
                            .push(ReplaceDefinitionError::InvalidPublicKey(tentative_definition.name, e));
                        continue;
                    }
                }
            }
            None => None,
        };

        if !nns_reachable(tentative_definition.nns_urls.clone()).await {
            error!(
                binding.log,
                "Submitted definition {} is not reachable", tentative_definition.name
            );
            error
                .errors
                .push(ReplaceDefinitionError::NNSUnreachable(tentative_definition.name));
            continue;
        }

        let (stop_signal_sender, stop_signal_rcv) = crossbeam::channel::bounded::<()>(0);
        let def = Definition::new(
            tentative_definition.nns_urls,
            binding.registry_path.clone(),
            tentative_definition.name,
            binding.log.clone(),
            public_key,
            binding.poll_interval,
            stop_signal_rcv,
            binding.registry_query_timeout,
            stop_signal_sender,
        );
        info!(binding.log, "Adding new definition {} to existing", def.name);
        existing_definitions.push(def);
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
    match _replace_definitions(definitions, binding).await {
        Ok(_) => Ok(warp::reply::with_status(
            "Definitions added successfully".to_string(),
            warp::http::StatusCode::OK,
        )),
        Err(error) => Ok(warp::reply::with_status(
            format!("Definitions could not be replaced:\n{}", error),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}
