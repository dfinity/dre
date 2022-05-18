use derive_builder::Builder;

use gitlab::api::common::{self, NameOrId};
use gitlab::api::endpoint_prelude::*;

/// Query for a specific commit in a project.
#[derive(Debug, Builder)]
#[builder(setter(strip_option))]
pub struct CommitRefs<'a> {
    /// The project to get a commit from.
    #[builder(setter(into))]
    project: NameOrId<'a>,
    /// The commit to get.
    #[builder(setter(into))]
    commit: Cow<'a, str>,

    /// Include commit stats.
    #[builder(default)]
    stats: Option<bool>,
}

impl<'a> CommitRefs<'a> {
    /// Create a builder for the endpoint.
    pub fn builder() -> CommitRefsBuilder<'a> {
        CommitRefsBuilder::default()
    }
}

impl<'a> Endpoint for CommitRefs<'a> {
    fn method(&self) -> Method {
        Method::GET
    }

    fn endpoint(&self) -> Cow<'static, str> {
        format!(
            "projects/{}/repository/commits/{}/refs",
            self.project,
            common::path_escaped(&self.commit),
        )
        .into()
    }

    fn parameters(&self) -> QueryParams {
        let mut params = QueryParams::default();

        params.push_opt("stats", self.stats);

        params
    }
}

impl<'a> Pageable for CommitRefs<'a> {
    fn use_keyset_pagination(&self) -> bool {
        false
    }
}
