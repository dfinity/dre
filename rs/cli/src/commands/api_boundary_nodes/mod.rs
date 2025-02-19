use add::Add;
use clap::Args;
use remove::Remove;
use update::Update;

use crate::ctx::exe::impl_executable_command_for_enums;

mod add;
mod remove;
mod update;

#[derive(Args, Debug)]
pub struct ApiBoundaryNodes {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { ApiBoundaryNodes, Add, Update, Remove }
