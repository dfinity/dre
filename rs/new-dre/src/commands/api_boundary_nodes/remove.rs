use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    ic_admin,
};

#[derive(Args, Debug)]
pub struct Remove {
    /// Node IDs of API BNs that should be turned into unassigned nodes again
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    /// Motivation for removing the API BNs
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,
}

impl ExecutableCommand for Remove {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let ic_admin = ctx.ic_admin();
        ic_admin
            .propose_run(
                ic_admin::ProposeCommand::RemoveApiBoundaryNodes { nodes: self.nodes.to_vec() },
                ic_admin::ProposeOptions {
                    title: Some(format!("Remove {} API boundary node(s)", self.nodes.len())),
                    summary: Some(format!("Remove {} API boundary node(s)", self.nodes.len())),
                    motivation: self.motivation.clone(),
                },
            )
            .await?;

        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {}
}
