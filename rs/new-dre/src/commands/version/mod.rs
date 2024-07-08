use clap::{Args, Subcommand};
use guest_os::GuestOs;
use host_os::HostOs;
use ic_management_types::Artifact;

use super::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

mod guest_os;
mod host_os;

#[derive(Args, Debug)]
pub struct VersionCmd {
    #[clap(subcommand)]
    pub subcommand: VersionCommands,
}

#[derive(Subcommand, Debug)]
pub enum VersionCommands {
    ReviseElectedVersions(ReviseElectedVersionsCmd),
}

impl ExecutableCommand for VersionCmd {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            VersionCommands::ReviseElectedVersions(c) => c.require_ic_admin(),
        }
    }

    fn require_registry(&self) -> RegistryRequirement {
        match &self.subcommand {
            VersionCommands::ReviseElectedVersions(c) => c.require_registry(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            VersionCommands::ReviseElectedVersions(c) => c.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            VersionCommands::ReviseElectedVersions(c) => c.validate(cmd),
        }
    }
}

#[derive(Args, Debug)]
pub struct ReviseElectedVersionsCmd {
    #[clap(subcommand)]
    pub subcommand: ReviseElectedVersionsCommands,
}

#[derive(Subcommand, Debug)]
pub enum ReviseElectedVersionsCommands {
    #[clap(about = r#"Update the elected/blessed GuestOS versions in the registry
by adding a new version and potentially removing obsolete
versions"#)]
    GuestOs(GuestOs),

    #[clap(about = r#"Update the elected/blessed HostOS versions in the registry
by adding a new version and potentially removing obsolete versions"#)]
    HostOs(HostOs),
}

impl From<ReviseElectedVersionsCommands> for Artifact {
    fn from(value: ReviseElectedVersionsCommands) -> Self {
        match value {
            ReviseElectedVersionsCommands::GuestOs { .. } => Artifact::GuestOs,
            ReviseElectedVersionsCommands::HostOs { .. } => Artifact::HostOs,
        }
    }
}

impl ExecutableCommand for ReviseElectedVersionsCmd {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        match &self.subcommand {
            ReviseElectedVersionsCommands::GuestOs(g) => g.require_ic_admin(),
            ReviseElectedVersionsCommands::HostOs(h) => h.require_ic_admin(),
        }
    }

    fn require_registry(&self) -> RegistryRequirement {
        match &self.subcommand {
            ReviseElectedVersionsCommands::GuestOs(g) => g.require_registry(),
            ReviseElectedVersionsCommands::HostOs(h) => h.require_registry(),
        }
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        match &self.subcommand {
            ReviseElectedVersionsCommands::GuestOs(g) => g.execute(ctx).await,
            ReviseElectedVersionsCommands::HostOs(h) => h.execute(ctx).await,
        }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        match &self.subcommand {
            ReviseElectedVersionsCommands::GuestOs(g) => g.validate(cmd),
            ReviseElectedVersionsCommands::HostOs(h) => h.validate(cmd),
        }
    }
}
