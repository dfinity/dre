use slog::{error, info, Logger};

use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use std::fmt::{Display, Error as FmtError, Formatter};
use warp::Reply;

use crate::definition::{DefinitionsSupervisor, StartDefinitionError};
use crate::server_handlers::dto::BadDtoError;
use crate::server_handlers::dto::DefinitionDto;
use crate::server_handlers::WebResult;

pub(crate) struct AddDefinitionBinding {
    pub supervisor: DefinitionsSupervisor,
    pub log: Logger,
    pub registry_path: PathBuf,
    pub poll_interval: Duration,
    pub registry_query_timeout: Duration,
}

#[derive(Debug)]
pub enum AddDefinitionError {
    StartDefinitionError(StartDefinitionError),
    BadDtoError(BadDtoError),
}

impl Error for AddDefinitionError {}

impl Display for AddDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::StartDefinitionError(e) => write!(f, "{}", e),
            Self::BadDtoError(e) => write!(f, "{}", e),
        }
    }
}

async fn _add_definition(
    tentative_definition: DefinitionDto,
    binding: AddDefinitionBinding,
) -> Result<(), AddDefinitionError> {
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
        Err(e) => return Err(AddDefinitionError::BadDtoError(e)),
    };

    match binding.supervisor.start(vec![new_definition], false).await {
        Ok(()) => Ok(()),
        Err(e) => Err(AddDefinitionError::StartDefinitionError(
            e.errors.into_iter().next().unwrap(),
        )),
    }
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
