mod commit_refs;
use gitlab::{AsyncGitlab, GitlabBuilder};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommitRef {
    #[serde(alias = "type")]
    pub kind: String,
    pub name: String,
}

pub async fn authenticated_client(env: &str) -> AsyncGitlab {
    GitlabBuilder::new(
        "gitlab.com",
        std::env::var(env).unwrap_or_else(|_| panic!("missing {} env variable", env)),
    )
    .build_async()
    .await
    .expect("unable to initialize gitlab token")
}
