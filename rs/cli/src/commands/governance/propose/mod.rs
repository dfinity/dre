use clap::Args;
mod motion;

use motion::Motion;

use crate::exe::impl_executable_command_for_enums;

#[derive(Args, Debug)]
/// Creation of proposals.
pub struct Propose {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Propose, Motion }
