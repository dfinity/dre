use clap::Args;
use execute::Execute;
use list::List;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

pub mod execute;
mod list;

#[derive(Args, Debug)]
pub struct Qualify {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Qualify, List, Execute }
