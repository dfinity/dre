use clap::Args;
mod propose;

use propose::Propose;

use crate::exe::impl_executable_command_for_enums;

#[derive(Args, Debug)]
/// Commands and actions related to governance.
pub struct Governance {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Governance, Propose }
