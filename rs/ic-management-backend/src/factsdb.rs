use gitlab::{api::AsyncQuery, AsyncGitlab};
use hyper::StatusCode;
use ic_management_types::{FactsDBGuest, Guest};
use log::warn;

pub async fn query_guests(gitlab_client: AsyncGitlab, network: String) -> anyhow::Result<Vec<Guest>> {
    ::gitlab::api::raw(
        ::gitlab::api::projects::repository::files::FileRaw::builder()
            .ref_("refs/heads/main")
            .project("dfinity-lab/core/release")
            .file_path(format!("factsdb/data/{}_guests.csv", network))
            .build()
            .expect("failed to build API endpoint"),
    )
    .query_async(&gitlab_client)
    .await
    .map(|r| {
        csv::Reader::from_reader(r.as_slice())
            .deserialize()
            .map(|r| {
                let fdbg: FactsDBGuest = r.expect("record failed to parse");
                Guest::from(fdbg)
            })
            .collect::<Vec<_>>()
    })
    .or_else(|e| match e {
        ::gitlab::api::ApiError::Gitlab { msg } if msg.starts_with(&StatusCode::NOT_FOUND.as_u16().to_string()) => {
            warn!("No factsdb guests file found for network {network}: {msg}");
            Ok(vec![])
        }
        _ => Err(anyhow::anyhow!(e)),
    })
}
