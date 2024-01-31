use slog::Logger;
use std::error::Error;
use std::fmt::{Display, Error as FmtError, Formatter};

use warp::Reply;

use crate::definition::DefinitionsSupervisor;
use crate::server_handlers::dto::BoundaryNodeDto;
use crate::server_handlers::{bad_request, not_found, ok, WebResult};

#[derive(Clone)]
pub(super) struct AddBoundaryNodeToDefinitionBinding {
    pub(crate) supervisor: DefinitionsSupervisor,
    pub(crate) log: Logger,
}

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
    boundary_node: BoundaryNodeDto,
    binding: AddBoundaryNodeToDefinitionBinding,
) -> WebResult<impl Reply> {
    let log = binding.log.clone();
    let name = boundary_node.name.clone();
    let ic_name = boundary_node.ic_name.clone();
    let rej: String = format!("Definition {} could not be added", name);

    let mut definitions = binding.supervisor.definitions.lock().await;

    let running_definition = match definitions.get_mut(&ic_name) {
        Some(d) => d,
        None => return not_found(log, rej, DefinitionNotFound { ic_name }),
    };

    let bn = match boundary_node.try_into_boundary_node() {
        Ok(bn) => bn,
        Err(e) => return bad_request(log, rej, e),
    };

    match running_definition.add_boundary_node(bn).await {
        Ok(()) => ok(log, format!("Definition {} added successfully", name)),
        Err(e) => bad_request(log, rej, e),
    }
}
