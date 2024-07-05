use clap::{Args, Subcommand};
use guest_os::GuestOs;
use host_os::HostOs;
use ic_management_types::Artifact;

mod guest_os;
mod host_os;

#[derive(Args, Debug)]
pub struct VersionCmd {
    #[clap(subcommand)]
    pub subcommand: VersionCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum VersionCommands {
    ReviseElectedVersions(ReviseElectedVersionsCmd),
}

#[derive(Args, Debug, Clone)]
pub struct ReviseElectedVersionsCmd {
    #[clap(subcommand)]
    pub subcommand: ReviseElectedVersionsCommands,
}

#[derive(Subcommand, Debug, Clone)]
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
