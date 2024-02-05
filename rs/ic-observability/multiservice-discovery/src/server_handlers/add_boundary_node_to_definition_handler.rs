use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};

use crate::server_handlers::dto::BoundaryNodeDto;

use super::Server;

#[derive(Debug)]

struct DefinitionNotFound {
    ic_name: String,
}

impl Error for DefinitionNotFound {}

impl Display for DefinitionNotFound {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "definition {} not found", self.ic_name)
    }
}

pub(super) async fn add_boundary_node(
    State(binding): State<Server>,
    Json(boundary_node): Json<BoundaryNodeDto>,
) -> Result<String, (StatusCode, String)> {
    let name = boundary_node.name.clone();
    let ic_name = boundary_node.ic_name.clone();

    let mut definitions = binding.supervisor.definitions.lock().await;

    let running_definition = match definitions.get_mut(&ic_name) {
        Some(d) => d,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("Couldn't find definition: '{}'", ic_name),
            ))
        }
    };

    let bn = match boundary_node.try_into_boundary_node() {
        Ok(bn) => bn,
        Err(e) => return Err((StatusCode::BAD_REQUEST, e.to_string())),
    };

    match running_definition.add_boundary_node(bn).await {
        Ok(()) => Ok(format!("Definition {} added successfully", name)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}
