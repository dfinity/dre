use super::{AuthRequirement, ExecutableCommand};
use clap::Args;
mod propose;

use propose::Propose;

#[derive(Args, Debug)]
/// Commands and actions related to governance.
pub struct Governance {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

super::impl_executable_command_for_enums! { Governance, Propose }
