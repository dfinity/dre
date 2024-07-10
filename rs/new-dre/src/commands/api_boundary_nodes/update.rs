use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement},
    ic_admin,
};

#[derive(Args, Debug)]
pub struct Update {
    /// Node IDs where to rollout the version
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    #[clap(long, required = true)]
    pub version: String,

    /// Motivation for creating the subnet
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,
}

impl ExecutableCommand for Update {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let ic_admin = ctx.ic_admin();

        ic_admin
            .propose_run(
                ic_admin::ProposeCommand::DeployGuestosToSomeApiBoundaryNodes {
                    nodes: self.nodes.to_vec(),
                    version: self.version.to_string(),
                },
                ic_admin::ProposeOptions {
                    title: Some(format!("Update {} API boundary node(s) to {}", self.nodes.len(), &self.version)),
                    summary: Some(format!("Update {} API boundary node(s) to {}", self.nodes.len(), &self.version)),
                    motivation: self.motivation.clone(),
                },
            )
            .await?;

        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {}
}
