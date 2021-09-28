use serde::{Serialize, Deserialize};
use clap::{Clap, AppSettings};
use std::str::FromStr;
use anyhow::{anyhow, Error};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecentralizedNodeQuery {
    pub subnet: String,
    pub removals: Option<Vec<String>>,
    pub node_count: i32,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodesToRemoveResponse {
    pub nodes: Vec<String>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BestNodesResponse {
    pub nodes: Vec<String>
}


