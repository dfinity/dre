use candid::Decode;
use futures::executor::block_on;
use ic_agent::Agent;
use ic_nns_governance::pb::v1::ProposalInfo;
use log::debug;

use std::env;

/// Return the configuration value from the environment.
pub fn env_cfg(key: &str) -> String {
    match env::var(key) {
        Ok(value) => value,
        Err(err) => panic!(
            "Environment variable `{}` is not set. Please update the .env file. {}",
            key, err
        ),
    }
}

#[derive(Debug, PartialEq)]
pub struct ProposalStatus {
    pub id: u64,
    pub summary: String,
    pub timestamp_seconds: u64,
    pub executed_timestamp_seconds: u64,
    pub failed_timestamp_seconds: u64,
    pub failure_reason: String,
}

struct ProposalPoller {
    agent: Agent,
}

impl ProposalPoller {
    fn new() -> Self {
        let agent = Agent::builder()
            .with_transport(
                ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport::create("https://nns.ic0.app")
                    .expect("failed to create transport"),
            )
            .build()
            .expect("failed to build the agent");
        Self { agent }
    }

    pub fn get_proposal_info(&self, proposal_id: u64) -> Option<ProposalInfo> {
        let response = block_on(
            self.agent
                .query(
                    &ic_types::Principal::from_slice(ic_nns_constants::GOVERNANCE_CANISTER_ID.get().as_slice()),
                    "get_proposal_info",
                )
                .with_arg(
                    candid::IDLArgs::new(&[candid::parser::value::IDLValue::Nat64(proposal_id)])
                        .to_bytes()
                        .unwrap()
                        .as_slice(),
                )
                .call(),
        )
        .expect("unable to query get_proposal_info");

        Decode!(response.as_slice(), Option<ProposalInfo>).expect("unable to decode")
    }
}

/// Get status of an NNS proposal
pub fn get_proposal_status(proposal_id: u64) -> Result<ProposalStatus, anyhow::Error> {
    debug!("get_proposal_status: {}", proposal_id);

    let proposal_poller = ProposalPoller::new();

    let proposal_info: ProposalInfo = proposal_poller.get_proposal_info(proposal_id).unwrap();

    Ok(ProposalStatus {
        id: proposal_info.id.unwrap().id,
        summary: match proposal_info.proposal {
            None => "".to_string(),
            Some(proposal) => proposal.summary,
        },
        timestamp_seconds: proposal_info.proposal_timestamp_seconds,
        executed_timestamp_seconds: proposal_info.executed_timestamp_seconds,
        failed_timestamp_seconds: proposal_info.failed_timestamp_seconds,
        failure_reason: match proposal_info.failure_reason {
            None => "".to_string(),
            Some(failure_reason) => format!("{:?}", failure_reason),
        },
    })
}

#[cfg(test)]
mod tests;
