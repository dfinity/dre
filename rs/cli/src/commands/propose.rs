use clap::Args;

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Propose {
    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ExecutableCommand for Propose {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let ic_admin = ctx.ic_admin().await?;

        ic_admin.run_passthrough_propose(&self.args).await?;
        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
