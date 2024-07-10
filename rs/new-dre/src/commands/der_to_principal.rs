use std::path::PathBuf;

use clap::Args;

use super::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
pub struct DerToPrincipal {
    /// Path to the DER file
    pub path: PathBuf,
}

impl ExecutableCommand for DerToPrincipal {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::None
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let principal = ic_base_types::PrincipalId::new_self_authenticating(&std::fs::read(&self.path)?);
        println!("{}", principal);
        Ok(())
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
