use clap::Args;
use ic_types::PrincipalId;

use super::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct TrustworthyMetrics {
    /// Wallet that should be used to query node metrics history
    /// in form of canister id
    pub wallet: String,

    /// Start at timestamp in nanoseconds
    pub start_at_timestamp: u64,

    /// Vector of subnets to query, if empty will dump metrics for
    /// all subnets
    pub subnet_ids: Vec<PrincipalId>,
}

impl ExecutableCommand for TrustworthyMetrics {
    fn require_neuron(&self) -> IcAdminRequirement {
        IcAdminRequirement::Detect
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }
}
