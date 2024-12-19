use clap::Args;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};
use crate::commands::version::revise::ReviseElectedVersions;

pub(crate) mod revise;

#[derive(Args, Debug)]
pub struct Version {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Version, ReviseElectedVersions }
