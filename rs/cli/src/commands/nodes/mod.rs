use clap::Args;
use remove::Remove;

use super::{impl_executable_command_for_enums, AuthRequirement, ExecutableCommand};

mod remove;

#[derive(Args, Debug)]
pub struct Nodes {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}
impl_executable_command_for_enums! { Nodes, Remove }
