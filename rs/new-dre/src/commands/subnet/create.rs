use clap::Args;
use ic_types::PrincipalId;

use crate::commands::{ExecutableCommand, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Create {
    /// Number of nodes in the subnet
    #[clap(long, default_value_t = 13)]
    pub size: usize,

    /// Minimum nakamoto coefficients desired
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

    /// Motivation for replacing custom nodes
    #[clap(long, short, aliases = [ "summary" ])]
    pub motivation: Option<String>,

    /// Replica version to use for the new subnet
    #[clap(long)]
    pub replica_version: Option<String>,

    /// Arbitrary other ic-args
    #[clap(allow_hyphen_values = true)]
    other_args: Vec<String>,

    /// Provide the list of all arguments that ic-admin accepts for subnet creation
    #[clap(long)]
    pub help_other_args: bool,
}

impl ExecutableCommand for Create {
    fn require_neuron(&self) -> bool {
        true
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
