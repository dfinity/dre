use add::Add;
use clap::Args;
use remove::Remove;
use update::Update;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

mod add;
mod remove;
mod update;

#[derive(Args, Debug)]
pub struct ApiBoundaryNodes {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { ApiBoundaryNodes, Add, Update, Remove }
