use clap::Args;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Get {
    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ExecutableCommand for Get {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let ic_admin = ctx.ic_admin();
        let _ = ic_admin.run_passthrough_get(&self.args, false).await?;

        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {}
}
