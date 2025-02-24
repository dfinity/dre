use crate::auth::AuthRequirement;
use crate::ctx::DreContext;
use clap::Command;
pub mod args;

pub trait ExecutableCommand {
    fn require_auth(&self) -> AuthRequirement;

    fn validate(&self, args: &args::GlobalArgs, cmd: &mut Command);

    fn execute(&self, ctx: DreContext) -> impl std::future::Future<Output = anyhow::Result<()>>;

    fn neuron_override(&self) -> Option<crate::auth::Neuron> {
        None
    }
}

macro_rules! impl_executable_command_for_enums {
    ($str_name:ident, $($var:ident),*) => {
        use crate::ctx::DreContext;
        use crate::exe::ExecutableCommand;
        use crate::auth::AuthRequirement;
        use crate::exe::args::GlobalArgs;
        use clap::{Subcommand, Command};

        #[derive(Subcommand, Debug)]
        pub enum Subcommands { $(
            $var($var),
        )*}

        impl ExecutableCommand for Subcommands {
            fn require_auth(&self) -> AuthRequirement {
                match &self {
                    $(Subcommands::$var(variant) => variant.require_auth(),)*
                }
            }

            async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
                match &self {
                    $(Subcommands::$var(variant) => variant.execute(ctx).await,)*
                }
            }

            fn validate(&self, args: &GlobalArgs, cmd: &mut Command) {
                match &self {
                    $(Subcommands::$var(variant) => variant.validate(args, cmd),)*
                }
            }

            fn neuron_override(&self) -> Option<crate::auth::Neuron> {
                match &self {
                    $(Subcommands::$var(variant) => variant.neuron_override(),)*
                }
            }
        }

        impl ExecutableCommand for $str_name {
            fn require_auth(&self) -> AuthRequirement {
                self.subcommands.require_auth()
            }

            async fn execute(&self, ctx: DreContext) -> anyhow::Result<()> {
                self.subcommands.execute(ctx).await
            }

            // Validate the command line arguments. You can return an error with something like:
            // ```rust
            // use clap::error::ErrorKind;
            // if args.neuron_id.is_none() {
            //    cmd.error(ErrorKind::MissingRequiredArgument, "Neuron ID is required for this command.").exit();
            // }
            // ```
            fn validate(&self, args: &GlobalArgs, cmd: &mut Command) {
                self.subcommands.validate(args, cmd)
            }

            fn neuron_override(&self) -> Option<crate::auth::Neuron> {
                self.subcommands.neuron_override()
            }
        }
    }
}

pub(crate) use impl_executable_command_for_enums;
