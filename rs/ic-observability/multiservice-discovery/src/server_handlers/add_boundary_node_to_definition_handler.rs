use std::str::FromStr;
use std::sync::Arc;

use service_discovery::job_types::JobType;
use slog::Logger;
use tokio::sync::Mutex;
use warp::Reply;

use crate::definition::{BoundaryNode, Definition};
use crate::server_handlers::dto::BoundaryNodeDto;
use crate::server_handlers::WebResult;

pub struct AddBoundaryNodeToDefinitionBinding {
    pub definitions: Arc<Mutex<Vec<Definition>>>,
    pub log: Logger,
}

pub async fn add_boundary_node(
    boundary_node: BoundaryNodeDto,
    binding: AddBoundaryNodeToDefinitionBinding,
) -> WebResult<impl Reply> {
    let mut definitions = binding.definitions.lock().await;

    let definition = match definitions.iter_mut().find(|d| d.name == boundary_node.ic_name) {
        Some(def) => def,
        None => {
            return Ok(warp::reply::with_status(
                "Definition with this name does not exist".to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    };

    if let Some(bn) = definition
        .boundary_nodes
        .iter()
        .find(|bn| bn.name == boundary_node.name)
    {
        return Ok(warp::reply::with_status(
            format!("Boundary node with name {} already exists", bn.name),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    };

    let job_type = match JobType::from_str(&boundary_node.job_type) {
        Err(e) => {
            // We don't have this job type here.
            return Ok(warp::reply::with_status(
                format!("Job type {} is not known: {}", boundary_node.job_type, e),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
        Ok(jt) => {
            // Forbid addition of any job type not known to be supported by boundary nodes.
            if !JobType::all_for_boundary_nodes().contains(&jt) {
                return Ok(warp::reply::with_status(
                    format!(
                        "Job type {} is not supported for boundary nodes.",
                        boundary_node.job_type
                    ),
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }
            jt
        }
    };

    definition.add_boundary_node(BoundaryNode {
        name: boundary_node.name,
        custom_labels: boundary_node.custom_labels,
        targets: boundary_node.targets,
        job_type,
    });

    Ok(warp::reply::with_status(
        "".to_string(),
        warp::http::StatusCode::CREATED,
    ))
}
