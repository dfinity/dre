use clap::{Args, Subcommand};
use execute::Execute;
use list::List;

use super::ExecutableCommand;

mod execute;
mod list;

#[derive(Args, Debug)]
pub struct QualifyCmd {
    #[clap(subcommand)]
    subcommand: QualifyCommands,
}

#[derive(Subcommand, Debug)]
enum QualifyCommands {
    /// List all steps present in the qualification
    List(List),
    /// Execute the qualification
    Execute(Execute),
}

impl ExecutableCommand for QualifyCmd {
    fn require_ic_admin(&self) -> super::IcAdminRequirement {
        match &self.subcommand {
            QualifyCommands::List(c) => c.require_ic_admin(),
            QualifyCommands::Execute(c) => c.require_ic_admin(),
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            QualifyCommands::List(c) => c.validate(cmd),
            QualifyCommands::Execute(c) => c.validate(cmd),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            QualifyCommands::List(c) => c.execute(ctx).await,
            QualifyCommands::Execute(c) => c.execute(ctx).await,
        }
    }
}
