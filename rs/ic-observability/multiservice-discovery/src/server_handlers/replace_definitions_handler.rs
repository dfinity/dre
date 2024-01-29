use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};

use slog::{error, info};

use futures::future::join_all;
use warp::Reply;

use crate::definition::{Definition, StartDefinitionError};
use crate::server_handlers::dto::{BadDtoError, DefinitionDto};
use crate::server_handlers::AddDefinitionBinding as ReplaceDefinitionsBinding;
use crate::server_handlers::WebResult;

#[derive(Debug)]
enum ReplaceDefinitionError {
    BadDtoError(BadDtoError),
    StartDefinitionError(StartDefinitionError),
}

impl Error for ReplaceDefinitionError {}

impl Display for ReplaceDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::BadDtoError(e) => write!(f, "{}", e),
            Self::StartDefinitionError(e) => write!(f, "{}", e),
        }
    }
}

impl From<BadDtoError> for ReplaceDefinitionError {
    fn from(e: BadDtoError) -> Self {
        Self::BadDtoError(e)
    }
}

impl From<StartDefinitionError> for ReplaceDefinitionError {
    fn from(e: StartDefinitionError) -> Self {
        Self::StartDefinitionError(e)
    }
}

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
    // Transform all definitions with checking.
    let defsresults_futures: Vec<_> = tentative_definitions
        .into_iter()
        .map(|tentative_definition| async {
            tentative_definition
                .try_into_definition(
                    binding.log.clone(),
                    binding.registry_path.clone(),
                    binding.poll_interval,
                    binding.registry_query_timeout,
                )
                .await
        })
        .collect();
    let defsresults: Vec<Result<Definition, BadDtoError>> = join_all(defsresults_futures).await;
    let (new_definitions, errors): (Vec<_>, Vec<_>) = defsresults.into_iter().partition(Result::is_ok);
    let new_definitions: Vec<_> = new_definitions.into_iter().map(Result::unwrap).collect();
    let errors: Vec<_> = errors.into_iter().map(Result::unwrap_err).collect();

    if !errors.is_empty() {
        return Err(ReplaceDefinitionsError {
            errors: errors.into_iter().map(ReplaceDefinitionError::from).collect(),
        });
    }

    match binding.supervisor.start(new_definitions, true).await {
        Ok(()) => Ok(()),
        Err(e) => Err(ReplaceDefinitionsError {
            errors: e.errors.into_iter().map(ReplaceDefinitionError::from).collect(),
        }),
    }
}

pub(crate) async fn replace_definitions(
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
