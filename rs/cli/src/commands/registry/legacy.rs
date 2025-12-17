use crate::{auth::AuthRequirement, exe::ExecutableCommand, exe::args::GlobalArgs};

#[derive(clap::Args, Debug)]
pub struct Legacy {}

impl ExecutableCommand for Legacy {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, _ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        // TODO: Implement legacy registry dump functionality
        println!("Legacy registry dump (to be implemented)");
        Ok(())
    }
}

