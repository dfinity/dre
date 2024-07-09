use clap::Args;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_management_types::filter_map_nns_function_proposals;
use ic_nns_governance::pb::v1::ProposalStatus;
use registry_canister::mutations::do_change_subnet_membership::ChangeSubnetMembershipPayload;

use crate::commands::{ExecutableCommand, IcAdminRequirement, RegistryRequirement};

#[derive(Args, Debug)]
pub struct Analyze {
    /// Proposal ID
    proposal_id: u64,
}

impl ExecutableCommand for Analyze {
    fn require_ic_admin(&self) -> IcAdminRequirement {
        IcAdminRequirement::Anonymous
    }

    fn require_registry(&self) -> RegistryRequirement {
        RegistryRequirement::WithNodeDetails
    }

    async fn execute(&self, ctx: crate::ctx::DreContext) -> anyhow::Result<()> {
        let client = GovernanceCanisterWrapper::from(ctx.create_canister_client()?);
        let proposal = client.get_proposal(self.proposal_id).await?;

        if proposal.status() != ProposalStatus::Open {
            return Err(anyhow::anyhow!(
                "Proposal {} has status {}\nProposal must have status: {}",
                self.proposal_id,
                proposal.status().as_str_name(),
                ProposalStatus::Open.as_str_name()
            ));
        }

        todo!("Implement once runner is migrated")
        // let runner = ctx.runner();

        // match filter_map_nns_function_proposals::<ChangeSubnetMembershipPayload>(&[proposal]).first() {
        //     Some((_, change_membership)) => runner.decentralization_change(change_membership).await,
        //     _ => Err(anyhow::anyhow!(
        //         "Proposal {} must have {} type",
        //         self.proposal_id,
        //         ic_nns_governance::pb::v1::NnsFunction::ChangeSubnetMembership.as_str_name()
        //     )),
        // }
    }

    fn validate(&self, cmd: &mut clap::Command) {
        ()
    }
}
