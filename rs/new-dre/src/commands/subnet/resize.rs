use clap::Args;
use ic_management_types::requests::SubnetResizeRequest;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct Resize {
    /// Number of nodes to be added
    #[clap(long)]
    pub add: usize,

    /// Number of nodes to be removed
    #[clap(long)]
    pub remove: usize,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    #[clap(long, num_args(1..), help = r#"Force t he inclusion of the provided nodes for replacement,
regardless of the decentralization score"#)]
    pub include: Vec<PrincipalId>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: String,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,
}

impl ExecutableCommand for Resize {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;
        runner
            .subnet_resize(
                SubnetResizeRequest {
                    subnet: self.id,
                    add: self.add,
                    remove: self.remove,
                    exclude: self.exclude.clone().into(),
                    only: self.only.clone().into(),
                    include: self.include.clone().into(),
                },
                self.motivation.clone(),
                todo!("Add support for global verbose flag"),
            )
            .await
    }

    fn validate(&self, cmd: &mut clap::Command) {}
}
