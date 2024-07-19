use clap::Args;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    qualification::{QualificationContext, QualificationExecutor},
};

#[derive(Args, Debug)]
pub struct List {
    /// Specify the steps to run
    /// A range can be: `4`, `3..`, `..3, `1..3`
    #[clap(long)]
    step_range: Option<String>,
}

impl ExecutableCommand for List {
    fn require_ic_admin(&self) -> crate::commands::IcAdminRequirement {
        IcAdminRequirement::None
    }

    fn validate(&self, _cmd: &mut clap::Command) {}

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let context = QualificationContext::new(ctx, self.step_range.clone().unwrap_or_default())
            .with_from_version("".to_string())
            .with_to_version("".to_string());
        let qualification_executor = QualificationExecutor::new(&context);
        qualification_executor.list();

        Ok(())
    }
}
