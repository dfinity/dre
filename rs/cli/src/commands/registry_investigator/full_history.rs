use crate::commands::registry_investigator::AuthRequirement;
use crate::exe::ExecutableCommand;
use crate::exe::args::GlobalArgs;
use clap::Args;

#[derive(Args, Debug)]
pub struct FullHistory {}

impl ExecutableCommand for FullHistory {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
