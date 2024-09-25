use super::{impl_executable_command_for_enums, ExecutableCommand};
use crate::commands::version::revise::guest_os::GuestOs;
use crate::commands::version::revise::host_os::HostOs;
use crate::commands::AuthRequirement;
use clap::Args;
use ic_management_types::Artifact;

pub(crate) mod guest_os;
pub(crate) mod host_os;

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
    fn require_auth(&self) -> AuthRequirement {
        self.subcommand.require_auth()
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        self.subcommand.execute(ctx).await
    }

    fn validate(&self, args: &crate::commands::Args, cmd: &mut clap::Command) -> Result<(), clap::Error> {
        self.subcommand.validate(args, cmd)
    }
}
