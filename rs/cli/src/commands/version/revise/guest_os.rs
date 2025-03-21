use clap::{error::ErrorKind, Args};

use crate::exe::args::GlobalArgs;
use crate::{
    auth::AuthRequirement,
    exe::ExecutableCommand,
    forum::ForumPostKind,
    submitter::{SubmissionParameters, Submitter},
};

#[derive(Debug, Args)]
pub struct GuestOs {
    /// Specify the commit hash of the version that is being elected
    #[clap(long)]
    pub version: String,

    /// Git tag for the release
    #[clap(long)]
    pub release_tag: Option<String>,

    /// Force proposal submission, ignoring missing download URLs
    #[clap(long, visible_alias = "force")]
    pub ignore_missing_urls: bool,

    /// Mark version as a security hotfix
    #[clap(long)]
    pub security_fix: bool,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
}

impl ExecutableCommand for GuestOs {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx
            .runner()
            .await?
            .do_revise_elected_replica_versions(
                &ic_management_types::Artifact::GuestOs,
                &self.version,
                &self.release_tag,
                self.ignore_missing_urls,
                self.security_fix,
            )
            .await?;
        Submitter::from(&self.submission_parameters)
            .propose_and_print(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        if self.submission_parameters.forum_parameters.forum_post_link_mandatory().is_err() {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Forum post link cannot be omitted for this subcommand.",
            )
            .exit()
        }
    }
}
