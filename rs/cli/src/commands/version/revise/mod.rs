use super::{impl_executable_command_for_enums, ExecutableCommand, IcAdminRequirement};
use crate::commands::version::revise::guest_os::GuestOs;
use crate::commands::version::revise::host_os::HostOs;
use clap::Args;
use ic_management_types::Artifact;

mod guest_os;
mod host_os;

#[derive(Args, Debug)]
pub struct ReviseElectedVersions {
    #[clap(subcommand)]
    pub subcommand: Subcommands,
}

impl_executable_command_for_enums! { GuestOs, HostOs }

impl From<Subcommands> for Artifact {
    fn from(value: Subcommands) -> Self {
        match value {
            Subcommands::GuestOs { .. } => Artifact::GuestOs,
            Subcommands::HostOs { .. } => Artifact::HostOs,
        }
    }
}

impl ExecutableCommand for ReviseElectedVersions {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        self.subcommand.require_ic_admin()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }

    fn validate(&self, cmd: &mut clap::Command) {
        self.subcommand.validate(cmd)
    }
}
