use clap::Args;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Debug, Args)]
pub struct HostOs {
    /// Specify the commit hash of the version that is being elected
    #[clap(long)]
    pub version: String,

    /// Git tag for the release
    #[clap(long)]
    pub release_tag: String,

    /// Force proposal submission, ignoring missing download URLs
    #[clap(long)]
    pub force: bool,
}

impl ExecutableCommand for HostOs {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;
        runner
            .do_revise_elected_replica_versions(&ic_management_types::Artifact::HostOs, &self.version, &self.release_tag, self.force)
            .await
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
