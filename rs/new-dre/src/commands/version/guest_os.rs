use clap::Args;

use crate::commands::ExecutableCommand;

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
}

impl ExecutableCommand for GuestOs {
    fn require_neuron(&self) -> bool {
        true
    }

    fn require_registry(&self) -> bool {
        false
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
