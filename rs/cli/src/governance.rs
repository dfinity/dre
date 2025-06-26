use futures::future::BoxFuture;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_governance_api::MakeProposalRequest;
use url::Url;

use crate::proposal_executors::{ProposableViaGovernanceCanister, ProposalExecution, ProposalResponseWithId};

pub struct GovernanceCanisterProposalExecutor {
    neuron_id: u64,
    governance_canister: GovernanceCanisterWrapper,
}

impl From<(u64, GovernanceCanisterWrapper)> for GovernanceCanisterProposalExecutor {
    fn from(args: (u64, GovernanceCanisterWrapper)) -> Self {
        Self {
            neuron_id: args.0,
            governance_canister: args.1,
        }
    }
}

impl GovernanceCanisterProposalExecutor {
    pub fn execution<T>(self, p: T) -> Box<dyn ProposalExecution>
    where
        T: 'static,
        T: ProposableViaGovernanceCanister<ProposalResult = ProposalResponseWithId>,
    {
        Box::new(ProposalExecutionViaGovernanceCanister { executor: self, proposal: p })
    }

    pub fn simulate<'c, 'd, W: ProposableViaGovernanceCanister + 'c>(
        &'d self,
        cmd: &'c W,
        forum_post_link_description: Option<String>,
    ) -> BoxFuture<'c, anyhow::Result<()>>
    where
        'd: 'c,
    {
        Box::pin(async move {
            println!("Proposal that would be submitted:\n{:#?}", cmd);
            println!("Forum post link: {}", forum_post_link_description.unwrap_or("None".to_string()));
            Ok(())
        })
    }

    pub fn submit<'c, 'd, W: ProposableViaGovernanceCanister + 'c>(
        &'d self,
        cmd: &'c W,
        forum_post_link: Option<Url>,
    ) -> BoxFuture<'c, anyhow::Result<ProposalResponseWithId>>
    where
        'd: 'c,
        <W as ProposableViaGovernanceCanister>::ProposalResult: TryInto<ProposalResponseWithId>,
        <W as ProposableViaGovernanceCanister>::ProposalResult: TryFrom<u64>,
    {
        Box::pin(async move {
            let response = self
                .governance_canister
                .make_proposal(
                    NeuronId { id: self.neuron_id },
                    MakeProposalRequest {
                        url: forum_post_link.map(|s| s.to_string()).unwrap_or_default(),
                        ..cmd.clone().into()
                    }
                    .into(),
                )
                .await?;
            let maybe_msg = response.message.clone();
            let pid: ProposalResponseWithId = response.try_into()?;
            if let Some(message) = maybe_msg {
                println!("{}", message);
            }
            Ok(pid)
        })
    }
}

struct ProposalExecutionViaGovernanceCanister<T> {
    executor: GovernanceCanisterProposalExecutor,
    proposal: T,
}

impl<T> ProposalExecution for ProposalExecutionViaGovernanceCanister<T>
where
    T: ProposableViaGovernanceCanister<ProposalResult = ProposalResponseWithId>,
{
    fn simulate(&self, forum_post_link_description: Option<String>) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async { self.executor.simulate(&self.proposal, forum_post_link_description).await })
    }

    fn submit<'a, 'b>(&'a self, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<ProposalResponseWithId>>
    where
        'a: 'b,
    {
        Box::pin(async { self.executor.submit(&self.proposal, forum_post_link).await })
    }
}
