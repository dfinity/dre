use crate::exe::args::GlobalArgs;
use clap::Args;
use ic_types::PrincipalId;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;

use crate::auth::AuthRequirement;
use crate::exe::ExecutableCommand;
#[derive(Args, Debug)]
#[clap(visible_aliases = &["analyze", "analyze-decentralization", "decentralization", "whatif", "what-if"])]
pub struct WhatifDecentralization {
    /// Set of nodes to add to the subnet in the analysis
    #[clap(long, num_args(1..))]
    pub add_nodes: Vec<PrincipalId>,

    /// Set of nodes to remove from the subnet in the analysis
    #[clap(long, num_args(1..))]
    pub remove_nodes: Vec<PrincipalId>,

    subnet_id: PrincipalId,

    /// List of initial nodes in the provided subnet,
    /// can be provided to override the current list of subnet nodes for the sake of analysis
    #[clap(long, num_args(1..))]
    subnet_nodes_initial: Option<Vec<PrincipalId>>,
}

impl ExecutableCommand for WhatifDecentralization {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await?;

        let change_membership = ChangeSubnetMembershipPayload {
            subnet_id: self.subnet_id,
            node_ids_add: self.add_nodes.iter().map(|id| (*id).into()).collect(),
            node_ids_remove: self.remove_nodes.iter().map(|id| (*id).into()).collect(),
        };

        let change = runner
            .decentralization_change(&change_membership, self.subnet_nodes_initial.clone(), None)
            .await?;

        println!("{}", change);
        Ok(())
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
