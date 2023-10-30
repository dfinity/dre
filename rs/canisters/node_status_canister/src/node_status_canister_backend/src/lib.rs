use std::{cell::RefCell, collections::BTreeMap};

use candid::{CandidType, Principal};
use serde::Deserialize;

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct NodeStatus {
    pub node_id: Principal,
    pub subnet_id: Option<Principal>,
    pub status: bool,
}

thread_local! {
    pub static STATUSES: RefCell<BTreeMap<Principal, NodeStatus>>  = RefCell::new(BTreeMap::new());
}

#[ic_cdk::query]
fn get_node_count() -> usize {
    STATUSES.with(|f| f.borrow().len())
}

#[ic_cdk::query]
fn get_node_status() -> Vec<NodeStatus> {
    STATUSES.with(|f| f.clone()).borrow().clone().into_values().collect()
}

#[ic_cdk::update]
fn update_node_status(new_statuses: Vec<NodeStatus>) -> bool {
    STATUSES.with(|f: &RefCell<BTreeMap<Principal, NodeStatus>>| {
        let mut statuses = f.borrow_mut();
        for new_status in new_statuses {
            statuses.insert(new_status.node_id, new_status);
        }
        true
    })
}
