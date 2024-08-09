use std::path::PathBuf;

use clap::{error::ErrorKind, Args};

use super::ExecutableCommand;

#[derive(Args, Debug)]
pub struct UpdateAuthorizedSubnets {
    path: PathBuf,
}

impl ExecutableCommand for UpdateAuthorizedSubnets {
    fn require_ic_admin(&self) -> super::IcAdminRequirement {
        super::IcAdminRequirement::Detect
    }

    fn validate(&self, cmd: &mut clap::Command) {
        if !self.path.exists() {
            cmd.error(ErrorKind::InvalidValue, format!("Path `{}` not found", self.path.display()))
                .exit();
        }

        if !self.path.is_file() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("Path `{}` found, but is not a file", self.path.display()),
            );
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        todo!()
    }
}
