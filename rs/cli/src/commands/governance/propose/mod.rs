use clap::Args;
mod motion;

use crate::commands::impl_executable_command_for_enums;

use motion::Motion;

#[derive(Args, Debug)]
/// Creation of proposals.
pub struct Propose {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Propose, Motion }
