use clap::{Args, Subcommand};

use crate::commands::registry::get::Get;
use crate::commands::registry::history::History;
use crate::commands::registry::diff::Diff;
use crate::commands::registry::legacy::Legacy;
use crate::exe::ExecutableCommand;
use crate::ctx::DreContext;
use crate::auth::AuthRequirement;
use crate::exe::args::GlobalArgs;

mod get;
mod history;
mod diff;
mod legacy;
mod helpers;

#[derive(Args, Debug)]
pub struct Registry {
    #[clap(subcommand)]
    pub subcommands: Option<Subcommands>,
}

#[derive(Subcommand, Debug)]
pub enum Subcommands {
    Get(Get),
    History(History),
    Diff(Diff),
}

// Manually implement ExecutableCommand to handle None case (legacy mode)
impl ExecutableCommand for Registry {
    fn require_auth(&self) -> AuthRequirement {
        match &self.subcommands {
            Some(sub) => sub.require_auth(),
            None => AuthRequirement::Anonymous,
        }
    }

    fn validate(&self, args: &GlobalArgs, cmd: &mut clap::Command) {
        if let Some(sub) = &self.subcommands {
            sub.validate(args, cmd);
        }
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        match &self.subcommands {
            Some(Subcommands::Get(get)) => get.execute(ctx).await,
            Some(Subcommands::History(history)) => history.execute(ctx).await,
            Some(Subcommands::Diff(diff)) => diff.execute(ctx).await,
            None => {
                // No subcommand => run legacy mode
                let legacy = Legacy {};
                legacy.execute(ctx).await
            }
        }
    }

    fn neuron_override(&self) -> Option<crate::auth::Neuron> {
        self.subcommands.as_ref().and_then(|s| s.neuron_override())
    }
}

impl ExecutableCommand for Subcommands {
    fn require_auth(&self) -> AuthRequirement {
        match self {
            Subcommands::Get(get) => get.require_auth(),
            Subcommands::History(history) => history.require_auth(),
            Subcommands::Diff(diff) => diff.require_auth(),
        }
    }

    fn validate(&self, args: &GlobalArgs, cmd: &mut clap::Command) {
        match self {
            Subcommands::Get(get) => get.validate(args, cmd),
            Subcommands::History(history) => history.validate(args, cmd),
            Subcommands::Diff(diff) => diff.validate(args, cmd),
        }
    }

    async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
        match self {
            Subcommands::Get(get) => get.execute(ctx).await,
            Subcommands::History(history) => history.execute(ctx).await,
            Subcommands::Diff(diff) => diff.execute(ctx).await,
        }
    }

    fn neuron_override(&self) -> Option<crate::auth::Neuron> {
        match self {
            Subcommands::Get(get) => get.neuron_override(),
            Subcommands::History(history) => history.neuron_override(),
            Subcommands::Diff(diff) => diff.neuron_override(),
        }
    }
}