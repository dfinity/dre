use std::str::FromStr;

use clap::{Args as ClapArgs, Parser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DryRunFormat {
    HumanReadable,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HowToProceed {
    Confirm,
    Unconditional,
    DryRun(DryRunFormat),
    #[allow(dead_code)]
    UnitTests, // Necessary for unit tests, otherwise confirmation is requested.
               // Generally this is hit when DreContext (created by get_mocked_ctx) has
               // both dry_run and proceed_without_confirmation set to true.
               // The net effect is that both the dry run and the final command are run.
}

#[derive(Debug, Clone, Parser)]
pub(crate) enum DryRunType {
    NoDryRun,
    HumanReadable,
    Json,
}

impl FromStr for DryRunType {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "no" => Ok(DryRunType::NoDryRun),
            "yes" => Ok(DryRunType::HumanReadable),
            "json" => Ok(DryRunType::Json),
            _ => {
                let mut cmd = clap::Command::new("dre");
                Err(cmd.error(clap::error::ErrorKind::InvalidValue, format!("invalid value {} for --dry-run", s)))
            }
        }
    }
}

/// Options for commands that may require confirmation.
#[derive(ClapArgs, Debug, Clone)]
pub struct ConfirmationModeOptions {
    /// To skip the confirmation prompt
    #[clap(
        short,
        long,
        global = true,
        env = "YES",
        conflicts_with = "dry_run",
        help_heading = "Options on how to proceed",
        help = "Do not ask for confirmation. If specified, the operation will be performed without requesting any confirmation from you."
    )]
    yes: bool,

    #[clap(long, aliases = [ "dry-run", "dryrun", "simulate", "no"], env = "DRY_RUN", global = true, conflicts_with = "yes", help = r#"Dry-run, or simulate operation. If specified will not make any changes; instead, it will show what would be done or submitted.  If specified as --dry-run=json, it will print machine-readable JSON to standard output."#, help_heading = "Options on how to proceed", num_args = 0..=1, default_value="no", default_missing_value="yes")]
    pub(crate) dry_run: DryRunType,
}

#[cfg(test)]
impl ConfirmationModeOptions {
    /// Return an option set for unit tests, not instantiable via command line due to conflict.
    pub fn for_unit_tests() -> Self {
        ConfirmationModeOptions {
            yes: true,
            dry_run: DryRunType::HumanReadable,
        }
    }
}

impl From<&ConfirmationModeOptions> for HowToProceed {
    fn from(o: &ConfirmationModeOptions) -> Self {
        match (&o.dry_run, o.yes) {
            (DryRunType::NoDryRun, true) => Self::Unconditional,
            (DryRunType::NoDryRun, false) => Self::Confirm,
            (DryRunType::HumanReadable, false) => Self::DryRun(DryRunFormat::HumanReadable),
            (DryRunType::Json, false) => Self::DryRun(DryRunFormat::Json),
            (_, true) => Self::UnitTests, // These variants cannot be instantiated via the command line.
        }
    }
}
