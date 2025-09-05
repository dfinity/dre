use clap::Args as ClapArgs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HowToProceed {
    Confirm,
    Unconditional,
    DryRun,
    #[allow(dead_code)]
    UnitTests, // Necessary for unit tests, otherwise confirmation is requested.
               // Generally this is hit when DreContext (created by get_mocked_ctx) has
               // both dry_run and proceed_without_confirmation set to true.
               // The net effect is that both the dry run and the final command are run.
}

#[derive(ClapArgs, Debug, Clone)]

/// Options for commands that may require confirmation.
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
    pub(crate) yes: bool,

    #[clap(long, aliases = [ "dry-run", "dryrun", "simulate", "no"], env = "DRY_RUN", global = true, conflicts_with = "yes", help = r#"Dry-run, or simulate operation. If specified will not make any changes; instead, it will show what would be done or submitted."#,help_heading = "Options on how to proceed")]
    pub(crate) dry_run: bool,
}

#[cfg(test)]
impl ConfirmationModeOptions {
    /// Return an option set for unit tests, not instantiable via command line due to conflict.
    pub fn for_unit_tests() -> Self {
        ConfirmationModeOptions { yes: true, dry_run: true }
    }
}

impl From<&ConfirmationModeOptions> for HowToProceed {
    fn from(o: &ConfirmationModeOptions) -> Self {
        match (o.dry_run, o.yes) {
            (false, true) => Self::Unconditional,
            (true, false) => Self::DryRun,
            (false, false) => Self::Confirm,
            (true, true) => Self::UnitTests, // This variant cannot be instantiated via the command line.
        }
    }
}
