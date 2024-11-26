use ic_nns_constants::CYCLES_MINTING_CANISTER_ID;
use ic_types::{CanisterId, PrincipalId};

use crate::IcAgentCanisterClient;

pub struct CyclesMintingCanisterWrapper {
    agent: IcAgentCanisterClient,
    canister_id: CanisterId,
}

impl From<IcAgentCanisterClient> for CyclesMintingCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self {
            agent: value,
            canister_id: CYCLES_MINTING_CANISTER_ID,
        }
    }
}

impl CyclesMintingCanisterWrapper {
    pub async fn get_authorized_subnets(&self) -> anyhow::Result<Vec<PrincipalId>> {
        self.agent
            .query(&self.canister_id.into(), "get_default_subnets", candid::encode_one(())?)
            .await
    }
}
