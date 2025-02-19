use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use futures::future::BoxFuture;
use ic_canisters::governance::GovernanceCanisterWrapper;
use ic_nns_common::pb::v1::NeuronId;
use ic_nns_governance_api::pb::v1::{manage_neuron_response::MakeProposalResponse, MakeProposalRequest};
use regex::Regex;
use url::Url;

use crate::ic_admin::IcAdmin;

/// A struct representing a response to a submitted proposal,
/// of the kind of responses that contain a proposal ID.
/// Not every proposal response from ic-admin can be deserialized
/// into a proposal ID, but most of them can.
pub struct ProposalResponseWithId(u64);

impl From<ProposalResponseWithId> for u64 {
    fn from(a: ProposalResponseWithId) -> Self {
        a.0
    }
}

impl TryFrom<u64> for ProposalResponseWithId {
    type Error = anyhow::Error;
    fn try_from(u: u64) -> Result<Self, Self::Error> {
        Ok(Self(u))
    }
}

impl TryFrom<String> for ProposalResponseWithId {
    type Error = anyhow::Error;

    fn try_from(response: String) -> Result<Self, Self::Error> {
        let last_line = response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .last()
            .ok_or(anyhow::anyhow!("Expected at least one line in the response"))?;
        let re = Regex::new(r"\s*(\d+)\s*")?;

        let captured = re
            .captures(&last_line.to_lowercase())
            .ok_or(anyhow::anyhow!("Expected some captures while parsing id from governance canister"))?
            .iter()
            .last()
            .ok_or(anyhow::anyhow!(
                "Expected at least one captures while parsing id from governance canister"
            ))?
            .ok_or(anyhow::anyhow!("Expected last element to be of type `Some()`"))?
            .as_str()
            .parse()
            .map_err(anyhow::Error::from)?;
        Ok(ProposalResponseWithId(captured))
    }
}

impl TryFrom<MakeProposalResponse> for ProposalResponseWithId {
    type Error = anyhow::Error;

    fn try_from(val: MakeProposalResponse) -> anyhow::Result<Self> {
        let propid = match (&val.proposal_id, &val.message) {
            (Some(proposal_id), _) => proposal_id.id,
            (None, Some(message)) => return Err(anyhow::anyhow!("Proposal submission failed: {}", message)),
            (None, None) => {
                return Err(anyhow::anyhow!(
                    "Proposal submission failed; no failure message was provided by the governance canister."
                ))
            }
        };
        Ok(Self(propid))
    }
}

/// Represents anything that can be turned into command line arguments
/// for ic-admin
/// The type Output is used to set which output type to expect and
/// deserialize when submitting a proposal.  Whatever the output
/// can be deserialized to using the Output TryFrom implementation,
/// it will be deserialized into (the caller decides the type).
/// Your proposal types can implement this to decode text ic_admin
/// output into strongly-typed structs.
pub trait RunnableViaIcAdmin: Send + Sync {
    type Output: TryFrom<String>;

    fn to_ic_admin_arguments(&self) -> anyhow::Result<Vec<String>>;
}

/// Represents a type of proposal that produces a concrete proposal
/// response with a proposal ID.
pub trait ProducesProposalResult {
    type ProposalResult: TryInto<ProposalResponseWithId>;
}

/// Represents anything that can be turned into a proposal request
/// suitable to be made through the GovernanceCanisterWrapper, and
/// also has the ability to deserialize to a ProposalResponseWithId.
pub trait ProposableViaGovernanceCanister: Debug + Send + Sync + Clone + Into<MakeProposalRequest> {
    type ProposalResult: TryFrom<u64> + TryInto<ProposalResponseWithId>;
}

/// It so happens that any MakeProposalRequest protobuf can be turned
/// into a governance canister proposal request.
impl ProposableViaGovernanceCanister for MakeProposalRequest {
    type ProposalResult = ProposalResponseWithId;
}

/// Knows how to simulate any RunnableViaIcAdmin, and submit any
/// of them too, returning the deserialized response based on the
/// ProducesProposalResult::Output type.
/// This is a higher-level construct than the IcAdmin trait, which
/// only concerns itself with raw proposal submission from arguments.
pub struct IcAdminProposalExecutor {
    ic_admin: Arc<dyn IcAdmin>,
}

impl From<Arc<dyn IcAdmin>> for IcAdminProposalExecutor {
    fn from(arg: Arc<dyn IcAdmin>) -> Self {
        Self { ic_admin: arg.clone() }
    }
}

impl IcAdminProposalExecutor {
    pub fn execution<T>(self, p: T) -> Box<dyn Execution>
    where
        T: 'static,
        T: RunnableViaIcAdmin<Output = ProposalResponseWithId>,
        T: ProducesProposalResult<ProposalResult = ProposalResponseWithId>,
    {
        Box::new(ProposalExecutionViaIcAdmin { executor: self, proposal: p })
    }

    pub fn run<'c, 'd, T: RunnableViaIcAdmin + 'c>(&'d self, cmd: &'c T, forum_post_link: Option<Url>) -> BoxFuture<'c, anyhow::Result<T::Output>>
    where
        'd: 'c,
        <<T as RunnableViaIcAdmin>::Output as TryFrom<String>>::Error: Display,
    {
        Box::pin(async move {
            let propose_command = cmd.to_ic_admin_arguments()?;
            let res = self.ic_admin.submit_proposal(propose_command, forum_post_link).await?;
            let parsed = T::Output::try_from(res.clone());
            parsed.map_err(|e| anyhow::anyhow!("Failed to deserialize result of proposal execution {}: {}", res, e))
        })
    }

    pub fn simulate<'c, 'd, T: RunnableViaIcAdmin + 'c>(&'d self, cmd: &'c T) -> BoxFuture<'c, anyhow::Result<()>>
    where
        'd: 'c,
    {
        Box::pin(async move {
            let propose_command = cmd.to_ic_admin_arguments()?;
            self.ic_admin.simulate_proposal(propose_command).await?;
            Ok(())
        })
    }

    pub fn submit<'c, 'd, U: ProducesProposalResult + RunnableViaIcAdmin + 'c>(
        &'d self,
        cmd: &'c U,
        forum_post_link: Option<Url>,
    ) -> BoxFuture<'c, anyhow::Result<ProposalResponseWithId>>
    where
        'd: 'c,
        <U as ProducesProposalResult>::ProposalResult: TryInto<ProposalResponseWithId>,
        <U as ProducesProposalResult>::ProposalResult: TryFrom<String>,
    {
        Box::pin(async move {
            let propose_command = cmd.to_ic_admin_arguments()?;
            let res = self.ic_admin.submit_proposal(propose_command, forum_post_link).await?;
            let parsed: anyhow::Result<ProposalResponseWithId> = ProposalResponseWithId::try_from(res.clone());
            parsed.map_err(|e| anyhow::anyhow!("Failed to deserialize result of proposal execution {}: {}", res, e))
        })
    }
}

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
    pub fn execution<T>(self, p: T) -> Box<dyn Execution>
    where
        T: 'static,
        T: ProposableViaGovernanceCanister<ProposalResult = ProposalResponseWithId>,
    {
        Box::new(ProposalExecutionViaGovernanceCanister { executor: self, proposal: p })
    }

    pub fn simulate<'c, 'd, W: ProposableViaGovernanceCanister + 'c>(&'d self, cmd: &'c W) -> BoxFuture<'c, anyhow::Result<()>>
    where
        'd: 'c,
    {
        Box::pin(async move {
            println!("Proposal that would be submitted:\n{:#?}", cmd);
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

/// Represents a single execution (either simulation or submission or both)
/// of a proposal (any object that implements either RunnableViaIcAdmin or
/// ProposableViaGovernanceCanister and produces a ProposalResponseWithId).
pub trait Execution: Send + Sync {
    fn simulate(&self) -> BoxFuture<'_, anyhow::Result<()>>;

    /// Runs the proposal in forrealz mode.  Result is returned and logged at debug level.
    fn submit<'a, 'b>(&'a self, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<ProposalResponseWithId>>
    where
        'a: 'b;
}

struct ProposalExecutionViaIcAdmin<T> {
    executor: IcAdminProposalExecutor,
    proposal: T,
}

impl<T> Execution for ProposalExecutionViaIcAdmin<T>
where
    T: RunnableViaIcAdmin<Output = ProposalResponseWithId>,
    T: ProducesProposalResult<ProposalResult = ProposalResponseWithId>,
{
    fn simulate(&self) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async { self.executor.simulate(&self.proposal).await })
    }

    fn submit<'a, 'b>(&'a self, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<ProposalResponseWithId>>
    where
        'a: 'b,
    {
        Box::pin(async { self.executor.submit(&self.proposal, forum_post_link).await })
    }
}

struct ProposalExecutionViaGovernanceCanister<T> {
    executor: GovernanceCanisterProposalExecutor,
    proposal: T,
}

impl<T> Execution for ProposalExecutionViaGovernanceCanister<T>
where
    T: ProposableViaGovernanceCanister<ProposalResult = ProposalResponseWithId>,
{
    fn simulate(&self) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async { self.executor.simulate(&self.proposal).await })
    }

    fn submit<'a, 'b>(&'a self, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<ProposalResponseWithId>>
    where
        'a: 'b,
    {
        Box::pin(async { self.executor.submit(&self.proposal, forum_post_link).await })
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {

    use super::*;

    #[test]
    fn parse_proposal_id_test() {
        let text = r#" some text blah 111
proposal 123456

"#;
        let parsed = ProposalResponseWithId::try_from(text.to_string()).unwrap();
        assert_eq!(parsed.0, 123456);

        let text = "222222";
        let parsed = ProposalResponseWithId::try_from(text.to_string()).unwrap();
        assert_eq!(parsed.0, 222222);

        let text = "Proposal id 123456";
        let parsed = ProposalResponseWithId::try_from(text.to_string()).unwrap();
        assert_eq!(parsed.0, 123456)
    }
}
