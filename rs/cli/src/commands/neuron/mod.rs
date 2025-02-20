use clap::Args;

use crate::commands::neuron::balance::Balance;
use crate::commands::neuron::refresh::Refresh;
use crate::commands::neuron::top_up::TopUp;
use crate::exe::impl_executable_command_for_enums;

mod balance;
mod refresh;
mod top_up;

#[derive(Args, Debug)]
pub struct Neuron {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Neuron, Balance, TopUp, Refresh }
