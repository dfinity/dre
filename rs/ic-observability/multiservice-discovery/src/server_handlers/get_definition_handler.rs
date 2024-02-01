use warp::reply::json;
use warp::Reply;

use crate::definition::DefinitionsSupervisor;
use crate::server_handlers::dto::DefinitionDto;
use crate::server_handlers::WebResult;

pub(super) async fn get_definitions(supervisor: DefinitionsSupervisor) -> WebResult<impl Reply> {
    let definitions = supervisor.definitions.lock().await;

    let list = &definitions
        .iter()
        .map(|(_, d)| {
            let x = &d.definition;
            x.into()
        })
        .collect::<Vec<DefinitionDto>>();
    Ok(json(list))
}
