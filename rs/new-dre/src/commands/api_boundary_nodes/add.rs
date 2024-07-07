use clap::Args;
use ic_types::PrincipalId;

use crate::{
    commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement},
    ic_admin,
};

#[derive(Args, Debug)]
pub struct Add {
    /// Node IDs to turn into API BNs
    #[clap(long, num_args(1..), required = true)]
    pub nodes: Vec<PrincipalId>,

    /// guestOS version
    #[clap(long, required = true)]
    pub version: String,

    /// Motivation for creating the subnet
    #[clap(short, long, aliases = ["summary"], required = true)]
    pub motivation: Option<String>,
}

impl ExecutableCommand for Add {
    fn require_neuron(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let ic_admin = ctx.ic_admin();

        ic_admin
            .propose_run(
                ic_admin::ProposeCommand::AddApiBoundaryNodes {
                    nodes: self.nodes.to_vec(),
                    version: self.version.clone(),
                },
                ic_admin::ProposeOptions {
                    title: Some(format!("Add {} API boundary node(s)", self.nodes.len())),
                    summary: Some(format!("Add {} API boundary node(s)", self.nodes.len())),
                    motivation: self.motivation.clone(),
                },
            )
            .await?;

        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
