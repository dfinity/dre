use std::hash::Hash;

use ic_sns_wasm::pb::v1::DeployedSns;

pub struct Sns {
    pub root_canister_id: String,
    pub governance_canister_id: String,
    pub ledger_canister_id: String,
    pub swap_canister_id: String,
    pub index_canister_id: String,
    pub name: String,
}

impl From<DeployedSns> for Sns {
    fn from(value: DeployedSns) -> Self {
        Self {
            governance_canister_id: match value.governance_canister_id {
                None => "".to_string(),
                Some(val) => val.to_string(),
            },
            index_canister_id: match value.index_canister_id {
                None => "".to_string(),
                Some(val) => val.to_string(),
            },
            ledger_canister_id: match value.ledger_canister_id {
                None => "".to_string(),
                Some(val) => val.to_string(),
            },
            root_canister_id: match value.root_canister_id {
                None => "".to_string(),
                Some(val) => val.to_string(),
            },
            swap_canister_id: match value.swap_canister_id {
                None => "".to_string(),
                Some(val) => val.to_string(),
            },
            name: "".to_string(),
        }
    }
}

impl Hash for Sns {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.root_canister_id.hash(state);
        self.governance_canister_id.hash(state);
        self.ledger_canister_id.hash(state);
        self.swap_canister_id.hash(state);
        self.index_canister_id.hash(state);
        self.name.hash(state);
    }
}
