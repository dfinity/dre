use clap::Args;

use crate::commands::version::revise::ReviseElectedVersions;
use crate::ctx::exe::impl_executable_command_for_enums;

pub(crate) mod revise;

#[derive(Args, Debug)]
pub struct Version {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Version, ReviseElectedVersions }
