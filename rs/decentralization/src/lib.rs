pub mod nakamoto;
pub mod network;
use std::collections::HashMap;

use ic_base_types::PrincipalId;
use nakamoto::Decentralize;
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct SubnetChangeResponse {
    pub added: Vec<PrincipalId>,
    pub removed: Vec<PrincipalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<PrincipalId>,
    pub score_before: nakamoto::NakamotoScore,
    pub score_after: nakamoto::NakamotoScore,
    pub motivation: Option<String>,
    pub feature_diff: HashMap<nakamoto::Feature, FeatureDiff>,
}

pub type FeatureDiff = HashMap<String, (usize, usize)>;

impl SubnetChangeResponse {
    pub fn with_motivation(self, motivation: String) -> Self {
        SubnetChangeResponse {
            motivation: Some(motivation),
            ..self
        }
    }
}

impl From<network::SubnetChange> for SubnetChangeResponse {
    fn from(change: network::SubnetChange) -> Self {
        Self {
            added: change.added().iter().map(|n| n.id).collect(),
            removed: change.removed().iter().map(|n| n.id).collect(),
            subnet_id: if change.id == Default::default() {
                None
            } else {
                Some(change.id)
            },
            score_before: nakamoto::NakamotoScore::from_vec_nodes(change.old_nodes.clone()),
            score_after: nakamoto::NakamotoScore::from_vec_nodes(change.new_nodes.clone()),
            motivation: None,
            feature_diff: change.new_nodes.iter().fold(
                change.old_nodes.iter().fold(
                    nakamoto::Feature::variants()
                        .into_iter()
                        .map(|f| (f, FeatureDiff::new()))
                        .collect::<HashMap<nakamoto::Feature, FeatureDiff>>(),
                    |mut acc, n| {
                        for f in nakamoto::Feature::variants() {
                            acc.get_mut(&f).unwrap().entry(n.get_feature(f)).or_insert((0, 0)).0 += 1;
                        }
                        acc
                    },
                ),
                |mut acc, n| {
                    for f in nakamoto::Feature::variants() {
                        acc.get_mut(&f).unwrap().entry(n.get_feature(f)).or_insert((0, 0)).1 += 1;
                    }
                    acc
                },
            ),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ReplaceRequest {
    pub nodes: Vec<PrincipalId>,
}

#[derive(Serialize, Deserialize)]
pub struct OptimizeQuery {
    pub max_replacements: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubnetCreateRequest {
    pub size: usize,
}
