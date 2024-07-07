use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Replace {
    /// Set of custom nodes to be replaced
    #[clap(long, short)]
    pub nodes: Vec<PrincipalId>,

    /// Do not replace unhealthy nodes
    #[clap(long)]
    pub no_heal: bool,

    #[clap(
        long,
        short,
        help = r#"Amount of nodes to be replaced by decentralization optimization 
algorithm"#
    )]
    pub optimize: Option<usize>,

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: Option<String>,

    /// Minimum Nakamoto coefficients after the replacement
    #[clap(long, num_args(1..))]
    pub min_nakamoto_coefficients: Vec<String>,

    /// Features or Node IDs to exclude from the available nodes pool
    #[clap(long, num_args(1..))]
    pub exclude: Vec<String>,

    /// Features or node IDs to only choose from
    #[clap(long, num_args(1..))]
    pub only: Vec<String>,

    #[clap(long, num_args(1..), help = r#"Force t he inclusion of the provided nodes for replacement,
regardless of the decentralization score"#)]
    pub include: Vec<PrincipalId>,

    /// The ID of the subnet.
    #[clap(long, short)]
    pub id: Option<PrincipalId>,
}

impl ExecutableCommand for Replace {
    fn require_neuron(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::WithNodeDetails
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
