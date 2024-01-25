use base64::{engine::general_purpose as b64, Engine as _};
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use ic_crypto_utils_threshold_sig_der::parse_threshold_sig_key_from_der;
use service_discovery::registry_sync::nns_reachable;
use slog::{error, Logger};
use tokio::sync::Mutex;
use warp::Reply;

use crate::definition::{wrap, Definition};
use crate::server_handlers::dto::DefinitionDto;
use crate::server_handlers::WebResult;

pub struct AddDefinitionBinding {
    pub definitions: Arc<Mutex<Vec<Definition>>>,
    pub log: Logger,
    pub registry_path: PathBuf,
    pub poll_interval: Duration,
    pub registry_query_timeout: Duration,
    pub rt: tokio::runtime::Handle,
    pub handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

#[derive(Debug)]
pub(crate) enum AddDefinitionError {
    InvalidPublicKey(String, std::io::Error),
    AlreadyExists(String),
    NNSUnreachable(String),
}

impl Error for AddDefinitionError {}

impl Display for AddDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::InvalidPublicKey(name, e) => {
                write!(f, "public key of definition {} is invalid: {}", name, e)
            }
            Self::AlreadyExists(name) => write!(f, "definition {} already exists", name),
            Self::NNSUnreachable(name) => {
                write!(f, "cannot reach any of the NNS nodes specified in definition {}", name)
            }
        }
    }
}

async fn _add_definition(definition: DefinitionDto, binding: AddDefinitionBinding) -> Result<(), AddDefinitionError> {
    let public_key = match definition.public_key {
        Some(pk) => {
            let decoded = b64::STANDARD.decode(pk).unwrap();

            match parse_threshold_sig_key_from_der(&decoded) {
                Ok(key) => Some(key),
                Err(e) => {
                    error!(
                        binding.log,
                        "Submitted definition {} has invalid public key", definition.name
                    );
                    return Err(AddDefinitionError::InvalidPublicKey(definition.name, e));
                }
            }
        }
        None => None,
    };

    let mut definitions = binding.definitions.lock().await;

    if definitions.iter().any(|d| d.name == definition.name) {
        error!(binding.log, "Submitted definition {} already exists", definition.name);
        return Err(AddDefinitionError::AlreadyExists(definition.name));
    }

    if !nns_reachable(definition.nns_urls.clone()).await {
        error!(binding.log, "Submitted definition {} is not reachable", definition.name);
        return Err(AddDefinitionError::NNSUnreachable(definition.name));
    }

    let (stop_signal_sender, stop_signal_rcv) = crossbeam::channel::bounded::<()>(0);
    let definition = Definition::new(
        definition.nns_urls,
        binding.registry_path.clone(),
        definition.name.clone(),
        binding.log,
        public_key,
        binding.poll_interval,
        stop_signal_rcv,
        binding.registry_query_timeout,
        stop_signal_sender,
    );

    definitions.push(definition.clone());

    let ic_handle = std::thread::spawn(wrap(definition, binding.rt));
    let mut handles = binding.handles.lock().await;
    handles.push(ic_handle);

    Ok(())
}

pub async fn add_definition(definition: DefinitionDto, binding: AddDefinitionBinding) -> WebResult<impl Reply> {
    let dname = definition.name.clone();
    match _add_definition(definition, binding).await {
        Ok(_) => Ok(warp::reply::with_status(
            format!("Definition {} added successfully", dname),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            format!("Definition {} could not be added: {}", dname, e),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}
