use std::fmt::{self, Debug, Display};

use futures::future::BoxFuture;
use ic_nns_governance_api::{manage_neuron_response::MakeProposalResponse, MakeProposalRequest};
use regex::Regex;
use url::Url;

/// A struct representing a response to a submitted proposal,
/// of the kind of responses that contain a proposal ID.
/// Not every proposal response from ic-admin can be deserialized
/// into a proposal ID, but most of them can.
#[derive(Clone)]
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

impl Display for ProposalResponseWithId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "proposal {}", self.0)
    }
}

impl TryFrom<String> for ProposalResponseWithId {
    type Error = anyhow::Error;

    fn try_from(response: String) -> Result<Self, Self::Error> {
        let last_line = response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .next_back()
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
pub trait ProposableViaGovernanceCanister: Debug + Send + Sync + Clone + Into<MakeProposalRequest> + serde::Serialize {
    type ProposalResult: TryFrom<u64> + TryInto<ProposalResponseWithId>;
}

/// It so happens that any MakeProposalRequest protobuf can be turned
/// into a governance canister proposal request.
impl ProposableViaGovernanceCanister for MakeProposalRequest {
    type ProposalResult = ProposalResponseWithId;
}

/// Represents a single execution (either simulation or submission or both)
/// of a proposal (any object that implements either RunnableViaIcAdmin or
/// ProposableViaGovernanceCanister, and produces a ProposalResponseWithId).
pub trait ProposalExecution: Send + Sync {
    fn simulate(&self, machine_readable: bool, forum_post_link_description: Option<String>) -> BoxFuture<'_, anyhow::Result<()>>;

    /// Runs the proposal in forrealz mode.  Result is returned and logged at debug level.
    fn submit<'a, 'b>(&'a self, forum_post_link: Option<Url>) -> BoxFuture<'b, anyhow::Result<ProposalResponseWithId>>
    where
        'a: 'b;
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
