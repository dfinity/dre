use crate::exe::impl_executable_command_for_enums;
use clap::Args;
use full_history::FullHistory;

mod full_history;

#[derive(Args, Debug)]
pub struct RegistryInvestigator {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! {
    RegistryInvestigator, FullHistory
}
