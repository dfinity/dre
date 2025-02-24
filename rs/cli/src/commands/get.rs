use clap::Args;

use crate::auth::AuthRequirement;
use crate::exe::{args::GlobalArgs, ExecutableCommand};

#[derive(Args, Debug)]
pub struct Get {
    /// Arbitrary ic-admin args
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl ExecutableCommand for Get {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        ctx.get(&self.args).await
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
