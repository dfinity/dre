use futures::future::join_all;
use warp::Reply;

use super::{bad_request, ok, WebResult};

use crate::definition::Definition;
use crate::server_handlers::dto::{BadDtoError, DefinitionDto};
use crate::server_handlers::AddDefinitionBinding as ReplaceDefinitionsBinding;

pub(crate) async fn replace_definitions(
    definitions: Vec<DefinitionDto>,
    binding: ReplaceDefinitionsBinding,
) -> WebResult<impl Reply> {
    let log = binding.log.clone();
    let rej = "Definitions could not be changed".to_string();
    let dnames = definitions
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<String>>()
        .join(", ");

    let defsresults_futures: Vec<_> = definitions
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
        return bad_request(
            log,
            rej,
            format!(
                ":\n * {}",
                errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n * ")
            ),
        );
    }

    match binding.supervisor.start(new_definitions, true).await {
        Ok(_) => ok(log, format!("Added new definitions {} to existing ones", dnames)),
        Err(e) => bad_request(log, rej, format!(":\n{}", e)),
    }
}
