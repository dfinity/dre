use clap::Parser;
use create::Create;
use deploy::Deploy;
use replace::Replace;
use rescue::Rescue;
use resize::Resize;
use set_authorization::SetAuthorization;
use whatif::WhatifDecentralization;

use crate::exe::impl_executable_command_for_enums;

mod create;
mod deploy;
mod replace;
mod rescue;
mod resize;
mod set_authorization;
mod whatif;

#[derive(Parser, Debug)]
pub struct Subnet {
    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { Subnet, WhatifDecentralization, Deploy, Replace, Resize, Create, Rescue, SetAuthorization }
