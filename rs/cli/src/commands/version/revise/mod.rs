use crate::commands::version::revise::guest_os::GuestOs;
use crate::commands::version::revise::host_os::HostOs;
use crate::ctx::exe::impl_executable_command_for_enums;
use clap::Args;
use ic_management_types::Artifact;

pub(crate) mod guest_os;
pub(crate) mod host_os;

#[derive(Args, Debug)]
pub struct ReviseElectedVersions {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { ReviseElectedVersions, GuestOs, HostOs }

impl From<Subcommands> for Artifact {
    fn from(value: Subcommands) -> Self {
        match value {
            Subcommands::GuestOs { .. } => Artifact::GuestOs,
            Subcommands::HostOs { .. } => Artifact::HostOs,
        }
    }
}
