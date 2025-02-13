use clap::{error::ErrorKind, Args};

use crate::{
    commands::{AuthRequirement, ExecutableCommand},
    forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind, ForumPostLinkVariant},
};

#[derive(Debug, Args)]
pub struct HostOs {
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
    pub forum_parameters: ForumParameters,
}

impl ExecutableCommand for HostOs {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner_proposal = ctx
            .runner()
            .await?
            .do_revise_elected_replica_versions(
                &ic_management_types::Artifact::HostOs,
                &self.version,
                &self.release_tag,
                self.ignore_missing_urls,
                self.security_fix,
            )
            .await?;
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_with_possible_confirmation(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, cmd: &mut clap::Command) {
        if let ForumPostLinkVariant::Omit = self.forum_parameters.forum_post_link {
            cmd.error(
                ErrorKind::MissingRequiredArgument,
                "Forum post link cannot be omitted for this subcommand.",
            )
            .exit()
        }
    }
}
