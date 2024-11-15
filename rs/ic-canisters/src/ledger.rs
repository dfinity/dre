use ic_nns_constants::{GOVERNANCE_CANISTER_ID, LEDGER_CANISTER_ID};
use icrc_ledger_types::icrc1::account::Account;

use crate::IcAgentCanisterClient;

pub struct LedgerCanisterWrapper {
    agent: IcAgentCanisterClient,
}

impl From<IcAgentCanisterClient> for LedgerCanisterWrapper {
    fn from(value: IcAgentCanisterClient) -> Self {
        Self { agent: value }
    }
}

impl<T> From<(T, IcAgentCanisterClient)> for LedgerCanisterWrapper {
    fn from(value: (T, IcAgentCanisterClient)) -> Self {
        let (_, client) = value;
        Self { agent: client }
    }
}

impl LedgerCanisterWrapper {
    pub async fn get_account_id(&self, subaccount: Option<[u8; 32]>) -> anyhow::Result<Vec<u8>> {
        let args = Account {
            owner: GOVERNANCE_CANISTER_ID.into(),
            subaccount,
        };
        self.agent
            .query::<Vec<u8>>(&LEDGER_CANISTER_ID.into(), "account_identifier", candid::encode_one(args)?)
            .await
    }
}
