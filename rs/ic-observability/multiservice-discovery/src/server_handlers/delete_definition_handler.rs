use crate::definition::DefinitionsSupervisor;
use crate::server_handlers::WebResult;
use warp::Reply;

pub async fn delete_definition(name: String, supervisor: DefinitionsSupervisor) -> WebResult<impl Reply> {
    if name == "ic" {
        return Ok(warp::reply::with_status(
            "Cannot delete ic definition".to_string(),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    match supervisor.stop(vec![name.clone()]).await {
        Ok(_) => Ok(warp::reply::with_status(
            format!("Deleted definition {}", name.clone()),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            format!("Could not delete definition {}: {}", name, e),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}
