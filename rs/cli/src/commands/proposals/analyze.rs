use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_management_types::filter_map_nns_function_proposals;
use ic_nns_governance::pb::v1::ProposalStatus;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;

use crate::auth::AuthRequirement;
use crate::exe::args::GlobalArgs;
use crate::exe::ExecutableCommand;

#[derive(Args, Debug)]
pub struct Analyze {
    /// Proposal ID
    proposal_id: u64,
}

impl ExecutableCommand for Analyze {
    fn require_auth(&self) -> AuthRequirement {
        AuthRequirement::Anonymous
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_ic_agent_canister_client().await?);
        let proposal = client.get_proposal(self.proposal_id).await?;
        let status = ProposalStatus::try_from(proposal.status)?;

        if status != ProposalStatus::Open {
            return Err(anyhow::anyhow!(
                "Proposal {} has status {}\nProposal must have status: {}",
                self.proposal_id,
                status.as_str_name(),
                ProposalStatus::Open.as_str_name()
            ));
        }
        let proposal_summary = proposal.clone().proposal.map(|p| p.summary);

        let runner = ctx.runner().await?;

        match filter_map_nns_function_proposals::<ChangeSubnetMembershipPayload>(&[proposal]).first() {
            Some((_, change_membership)) => runner.decentralization_change(change_membership, None, proposal_summary).await,
            _ => Err(anyhow::anyhow!(
                "Proposal {} must have {} type",
                self.proposal_id,
                ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetMembership.as_str_name()
            )),
        }
    }

    fn validate(&self, _args: &GlobalArgs, _cmd: &mut clap::Command) {}
}
