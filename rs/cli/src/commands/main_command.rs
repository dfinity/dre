use super::api_boundary_nodes::ApiBoundaryNodes;
use super::der_to_principal::DerToPrincipal;
use super::firewall::Firewall;
use super::get::Get;
use super::governance::Governance;
use super::hostos::HostOs;
use super::network::Network;
use super::neuron::Neuron;
use super::node_metrics::NodeMetrics;
use super::nodes::Nodes;
use super::proposals::Proposals;
use super::propose::Propose;
use super::qualify::Qualify;
use super::registry::Registry;
use super::update_authorized_subnets::UpdateAuthorizedSubnets;
use super::update_unassigned_nodes::UpdateUnassignedNodes;
use super::upgrade::Upgrade;
use super::version::Version;
use super::vote::Vote;
use crate::commands::subnet::Subnet;
use crate::exe::impl_executable_command_for_enums;
use clap::Parser;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, author)]
pub struct MainCommand {
    #[clap(flatten)]
    pub global_args: GlobalArgs,

    #[clap(subcommand)]
    pub subcommands: Subcommands,
}

impl_executable_command_for_enums! { MainCommand, DerToPrincipal, Network, Subnet, Get, Propose, UpdateUnassignedNodes, Version, NodeMetrics, HostOs, Nodes, ApiBoundaryNodes, Vote, Registry, Firewall, Upgrade, Proposals, Completions, Qualify, UpdateAuthorizedSubnets, Neuron, Governance }

#[derive(Args, Debug)]
pub struct Completions {
    #[clap(long, short, default_value_t = Shell::Bash)]
    shell: Shell,
}

impl ExecutableCommand for Completions {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}

    async fn execute(&self, _ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let mut command = MainCommand::command();

        generate(self.shell, &mut command, "dre", &mut std::io::stdout());

        Ok(())
    }
}
