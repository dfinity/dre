use clap::Parser;
use create::Create;
use deploy::Deploy;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;
use whatif::WhatifDecentralization;

use crate::exe::impl_executable_command_for_enums;

mod create;
mod deploy;
mod replace;
mod rescue;
mod resize;
mod whatif;

#[derive(Parser, Debug)]
pub struct Subnet {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Subnet, WhatifDecentralization, Deploy, Replace, Resize, Create, Rescue }
