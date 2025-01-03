use std::str::FromStr;

use clap::Args;
use ic_canisters::registry::RegistryCanisterWrapper;
use ic_types::PrincipalId;

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct UpdateUnassignedNodes {
    /// NNS subnet id
    #[clap(long)]
    pub nns_subnet_id: Option<String>,
}

impl ExecutableCommand for UpdateUnassignedNodes {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Neuron
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let nns_subnet_id = match &self.nns_subnet_id {
            Some(n) => n.to_owned(),
            None => {
                let (_, canister_agent) = ctx.create_ic_agent_canister_client().await?;
                let registry_client = RegistryCanisterWrapper::new(canister_agent.agent);
                let subnet_list = registry_client.get_subnets().await?;
                subnet_list
                    .first()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow::anyhow!("No subnet found"))?
                    .clone()
            }
        };

        let runner = ctx.runner().await?;
        if let Some(runner_proposal) = runner
            .update_unassigned_nodes(&PrincipalId::from_str(&nns_subnet_id)?, ctx.forum_post_link())
            .await?
        {
            let ic_admin = ctx.ic_admin().await?;
            ic_admin.propose_run(runner_proposal.cmd, runner_proposal.opts).await?;
        }
        Ok(())
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
