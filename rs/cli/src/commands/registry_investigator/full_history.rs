use crate::commands::registry_investigator::AuthRequirement;
use crate::exe::ExecutableCommand;
use crate::exe::args::GlobalArgs;
use clap::{Args, ValueEnum};

#[derive(Args, Debug)]
pub struct FullHistory {
    #[clap(long)]
    key_type: KeyType,

    #[clap(long)]
    key_value: Option<String>,
}

impl ExecutableCommand for FullHistory {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate(&self, _args: &GlobalArgs, cmd: &mut clap::Command) {
        match self.key_type {
            KeyType::SubnetList | KeyType::NodeRewardsTable => return,
            KeyType::ApiBoundaryNode
            | KeyType::Node
            | KeyType::NodeOperator
            | KeyType::ReplicaVersion
            | KeyType::HostOsVersion
            | KeyType::Subnet
            | KeyType::DataCenter
                if self.key_value.is_none() => {}
            _ => return,
        }

        cmd.error(
            clap::error::ErrorKind::InvalidValue,
            format!("Value is mandatory with submitted key type"),
        )
        .exit();
    }
}

/// Supported key types
#[derive(Debug, Clone, ValueEnum)]
enum KeyType {
    SubnetList,

    NodeRewardsTable,

    #[clap(aliases = ["api-bn"])]
    ApiBoundaryNode,

    #[clap(aliases = ["n"])]
    Node,

    #[clap(aliases = ["no"])]
    NodeOperator,

    ReplicaVersion,

    HostOsVersion,

    #[clap(aliases = ["s"])]
    Subnet,

    #[clap(aliases = ["dc"])]
    DataCenter,
}
