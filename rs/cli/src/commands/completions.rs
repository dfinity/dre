use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Completions {
    #[clap(long, short, default_value_t = Shell::Bash)]
    shell: Shell,
}

impl ExecutableCommand for Completions {
    fn require_ic_admin(&self) -> super::IcAdminRequirement {
        super::IcAdminRequirement::None
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, _ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let mut command = super::Args::command();

        generate(self.shell, &mut command, "dre", &mut std::io::stdout());

        Ok(())
    }
}
