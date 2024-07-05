use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
pub struct DerToPrincipal {
    /// Path to the DER file
    pub path: PathBuf,
}
