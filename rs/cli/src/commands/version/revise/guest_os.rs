use clap::Args;

use crate::commands::{AuthRequirement, ExecutableCommand};

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
}

impl ExecutableCommand for GuestOs {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        let runner_proposal = runner
            .do_revise_elected_replica_versions(
                &ic_management_types::Artifact::GuestOs,
                &self.version,
                &self.release_tag,
                self.ignore_missing_urls,
                ctx.forum_post_link().unwrap(), // checked in validate()
                self.security_fix,
            )
            .await?;
        let ic_admin = ctx.ic_admin().await?;
        ic_admin.propose_run(runner_proposal.cmd, runner_proposal.opts).await?;
        Ok(())
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) {
        if args.forum_post_link.is_none() {
            cmd.error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "Forum post link is required for this command",
            )
            .exit()
        }
    }
}
