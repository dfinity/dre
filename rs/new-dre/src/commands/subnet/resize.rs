use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

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
    pub motivation: Option<String>,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: PrincipalId,
}

impl ExecutableCommand for Resize {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::WithNodeDetails
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
