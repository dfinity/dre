use clap::Args;
use execute::Execute;
use list::List;

use crate::ctx::exe::impl_executable_command_for_enums;

pub mod execute;
mod list;

#[derive(Args, Debug)]
pub struct Qualify {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Qualify, List, Execute }
