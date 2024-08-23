use clap::Args;
use ic_types::PrincipalId;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;

use crate::commands::{ExecutableCommand, IcAdminRequirement};

#[derive(Args, Debug)]
#[clap(visible_aliases = &["analyze", "analyze-decentralization", "decentralization", "whatif", "what-if"])]
pub struct WhatifDecentralization {
    /// Set of nodes to add to the subnet in the analysis
    #[clap(long)]
    pub add_nodes: Vec<PrincipalId>,

    /// Set of nodes to remove from the subnet in the analysis
    #[clap(long)]
    pub remove_nodes: Vec<PrincipalId>,

    subnet_id: PrincipalId,

    /// List of initial nodes in the provided subnet,
    /// can be provided to override the current list of subnet nodes for the sake of analysis
    #[clap(long, num_args(1..))]
    subnet_nodes_initial: Option<Vec<PrincipalId>>,
}

impl ExecutableCommand for WhatifDecentralization {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let runner = ctx.runner().await;

        let change_membership = ChangeSubnetMembershipPayload {
            subnet_id: self.subnet_id,
            node_ids_add: self.add_nodes.iter().map(|id| (*id).into()).collect(),
            node_ids_remove: self.remove_nodes.iter().map(|id| (*id).into()).collect(),
        };

        runner
            .decentralization_change(&change_membership, self.subnet_nodes_initial.clone())
            .await
    }

    fn validate(&self, _cmd: &mut clap::Command) {}
}
