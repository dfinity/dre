use clap::Args;
use remove::Remove;

use crate::exe::impl_executable_command_for_enums;
mod remove;

#[derive(Args, Debug)]
pub struct Nodes {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}
impl_executable_command_for_enums! { Nodes, Remove }
