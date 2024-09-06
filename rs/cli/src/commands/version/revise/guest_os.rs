use clap::Args;

use crate::commands::{AuthRequirement, ExecutableCommand};

#[derive(Debug, Args)]
pub struct GuestOs {
    /// Specify the commit hash of the version that is being elected
    #[clap(long)]
    pub version: String,

    /// Git tag for the release
    #[clap(long)]
    pub release_tag: String,

    /// Force proposal submission, ignoring missing download URLs
    #[clap(long)]
    pub force: bool,

    /// Mark version as a security hotfix
    #[clap(long)]
    pub security_fix: bool,
}

impl ExecutableCommand for GuestOs {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;
        runner
            .do_revise_elected_replica_versions(
                &ic_management_types::Artifact::GuestOs,
                &self.version,
                &self.release_tag,
                self.force,
                ctx.forum_post_link(),
                self.security_fix,
            )
            .await
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
