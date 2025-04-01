use std::collections::HashMap;

#[derive(candid::CandidType, candid::Deserialize, Clone, PartialEq)]
pub struct RewardsPerNodeProviderResponse {
    pub rewards_per_provider: HashMap<String, u64>,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct RewardPeriodArgs {
    pub start_ts: u64,
    pub end_ts: u64,
}
