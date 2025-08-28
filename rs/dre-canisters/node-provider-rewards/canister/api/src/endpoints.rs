use candid::{CandidType, Deserialize, Principal};
use ic_nervous_system_proto::pb::v1::Decimal;
use ic_node_rewards_canister_protobuf::pb::rewards_calculator::v1::DayUtc;

#[derive(CandidType, Deserialize)]
pub struct NodeDailyFR {
    pub node_id: Principal,
    pub daily_relative_fr: Vec<(DayUtc, Decimal)>,
}

#[derive(CandidType, Deserialize)]
pub struct SubnetNodesFR {
    pub subnet_id: Principal,
    pub subnet_fr: Decimal,
    pub nodes_daily_fr: Vec<NodeDailyFR>,
}

pub type GetNodesFRBySubnet = Result<Vec<SubnetNodesFR>, String>;
