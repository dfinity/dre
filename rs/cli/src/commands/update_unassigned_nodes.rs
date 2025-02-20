use std::str::FromStr;

use clap::Args;
use ic_canisters::registry::RegistryCanisterWrapper;
use ic_types::PrincipalId;

use crate::auth::AuthRequirement;
use crate::exe::{ExecutableCommand, args::GlobalArgs};
use crate::forum::ForumPostKind;
use crate::submitter::{SubmissionParameters, Submitter};

#[derive(Args, Debug)]
pub struct UpdateUnassignedNodes {
    /// NNS subnet id
    #[clap(long)]
    pub nns_subnet_id: Option<String>,

    #[clap(flatten)]
    pub submission_parameters: SubmissionParameters,
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

        let runner_proposal = match ctx
            .runner()
            .await?
            .update_unassigned_nodes(&PrincipalId::from_str(&nns_subnet_id)?)
            .await?
        {
            Some(runner_proposal) => runner_proposal,
            None => return Ok(()),
        };
        Submitter::from(&self.submission_parameters)
            .propose(ctx.ic_admin_executor().await?.execution(runner_proposal), ForumPostKind::Generic)
            .await
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
