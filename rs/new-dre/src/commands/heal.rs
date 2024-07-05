use clap::Args;

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Heal {
    #[clap(
        short,
        long,
        help = r#"Maximum number of nodes to be replaced per subnet.
Optimization will be performed automatically maximizing the decentralization and
minimizing the number of replaced nodes per subnet"#
    )]
    pub max_replaceable_nodes_per_sub: Option<usize>,
}

impl ExecutableCommand for Heal {
    fn require_neuron(&self) -> bool {
        true
    }

    fn require_registry(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
