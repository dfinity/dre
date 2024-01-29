use slog::{error, info, Logger};
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};

use warp::Reply;

use crate::definition::{BoundaryNodeAlreadyExists, DefinitionsSupervisor};
use crate::server_handlers::dto::BoundaryNodeDto;
use crate::server_handlers::WebResult;

use super::dto::BadBoundaryNodeDtoError;

pub struct AddBoundaryNodeToDefinitionBinding {
    pub supervisor: DefinitionsSupervisor,
    pub log: Logger,
}

#[derive(Debug)]
pub enum AddBoundaryNodeError {
    DefinitionNotFound(String),
    BoundaryNodeAlreadyExists(BoundaryNodeAlreadyExists),
    BadBoundaryNodeDtoError(BadBoundaryNodeDtoError),
}

impl Error for AddBoundaryNodeError {}

impl Display for AddBoundaryNodeError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            Self::DefinitionNotFound(name) => write!(f, "definition {} not found", name),
            Self::BoundaryNodeAlreadyExists(name) => write!(f, "boundary node {} already exists", name),
            Self::BadBoundaryNodeDtoError(e) => write!(f, "{}", e),
        }
    }
}

impl From<BadBoundaryNodeDtoError> for AddBoundaryNodeError {
    fn from(e: BadBoundaryNodeDtoError) -> Self {
        Self::BadBoundaryNodeDtoError(e)
    }
}

impl From<BoundaryNodeAlreadyExists> for AddBoundaryNodeError {
    fn from(e: BoundaryNodeAlreadyExists) -> Self {
        Self::BoundaryNodeAlreadyExists(e)
    }
}

pub async fn _add_boundary_node(
    boundary_node: BoundaryNodeDto,
    binding: AddBoundaryNodeToDefinitionBinding,
) -> Result<(), AddBoundaryNodeError> {
    let mut definitions = binding.supervisor.definitions.lock().await;

    let running_definition = definitions
        .get_mut(&boundary_node.ic_name)
        .ok_or(AddBoundaryNodeError::DefinitionNotFound(boundary_node.ic_name.clone()))?;

    let bn = boundary_node.try_into_boundary_node()?;

    Ok(running_definition.add_boundary_node(bn).await?)
}

pub(crate) async fn add_boundary_node(
    boundary_node: BoundaryNodeDto,
    binding: AddBoundaryNodeToDefinitionBinding,
) -> WebResult<impl Reply> {
    let log = binding.log.clone();
    let dname = boundary_node.ic_name.clone();
    match _add_boundary_node(boundary_node, binding).await {
        Ok(_) => {
            info!(log, "Added new boundary node to definition {}", dname);
            Ok(warp::reply::with_status(
                format!("Definition {} added successfully", dname),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            error!(log, "Boundary node could not be added: {}", e);
            Ok(warp::reply::with_status(
                format!("Boundary node could not be added: {}", e),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
