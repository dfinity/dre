use ic_agent::Agent;
use ic_nns_constants::GOVERNANCE_CANISTER_ID;
use serde::{self, Serialize};
use std::str::FromStr;
use url::Url;

#[derive(Clone, Serialize)]
pub struct GovernanceCanisterVersion {
    pub stringified_hash: String,
}

pub async fn governance_canister_version(nns_url: Url) -> Result<GovernanceCanisterVersion, anyhow::Error> {
    let canister_agent = Agent::builder()
        .with_transport(ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport::create(
            nns_url,
        )?)
        .build()?;

    let governance_canister_build = std::str::from_utf8(
        &canister_agent
            .read_state_canister_metadata(
                candid::Principal::from_str(&GOVERNANCE_CANISTER_ID.to_string())
                    .expect("failed to convert governance canister principal to candid type"),
                "git_commit_id",
            )
            .await?,
    )?
    .trim()
    .to_string();

    Ok(GovernanceCanisterVersion {
        stringified_hash: governance_canister_build,
    })
}
