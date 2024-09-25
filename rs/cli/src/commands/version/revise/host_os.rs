use clap::Args;

use crate::commands::{AuthRequirement, ExecutableCommand};

#[derive(Debug, Args)]
pub struct HostOs {
    /// Specify the commit hash of the version that is being elected
    #[clap(long)]
    pub version: String,

    /// Git tag for the release
    #[clap(long)]
    pub release_tag: String,

    /// Force proposal submission, ignoring missing download URLs
    #[clap(long, visible_alias = "force")]
    pub ignore_missing_urls: bool,

    /// Mark version as a security hotfix
    #[clap(long)]
    pub security_fix: bool,
}

impl ExecutableCommand for HostOs {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;
        runner
            .do_revise_elected_replica_versions(
                &ic_management_types::Artifact::HostOs,
                &self.version,
                &self.release_tag,
                self.ignore_missing_urls,
                ctx.forum_post_link().unwrap(), // checked in validate()
                self.security_fix,
            )
            .await
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) -> Result<(), clap::Error> {
        match args.forum_post_link {
            Some(_) => Ok(()),
            None => Err(cmd.error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "Forum post link is required for this command",
            )),
        }
    }
}
