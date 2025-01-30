use std::str::FromStr;

use clap::Args;
use ic_canisters::registry::RegistryCanisterWrapper;
use ic_types::PrincipalId;

use crate::forum::{ic_admin::forum_enabled_proposer, ForumParameters, ForumPostKind};

use super::{AuthRequirement, ExecutableCommand};

#[derive(Args, Debug)]
pub struct UpdateUnassignedNodes {
    /// NNS subnet id
    #[clap(long)]
    pub nns_subnet_id: Option<String>,

    #[clap(flatten)]
    pub forum_parameters: ForumParameters,
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
        // FIXME I think the solution to the mut runner_proposal thing
        // is to create a different type (structural type) of proposal
        // and change the type of the propose_run thing to require
        // the URL via structural types.
        let runner_proposal = match runner.update_unassigned_nodes(&PrincipalId::from_str(&nns_subnet_id)?, None).await? {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };
        forum_enabled_proposer(&self.forum_parameters, &ctx, ctx.ic_admin().await?)
            .propose_run(runner_proposal.cmd, runner_proposal.opts, ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &crate::commands::Args, _cmd: &mut clap::Command) {}
}
