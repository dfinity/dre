use clap::Args;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    qualification::QualificationExecutor,
};

#[derive(Args, Debug)]
pub struct List {}

impl ExecutableCommand for List {
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        IcAdminRequirement::None
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, _ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let qualification_executor = QualificationExecutor::with_steps();
        qualification_executor.list();

        Ok(())
    }
}
