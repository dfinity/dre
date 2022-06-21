use crate::network::Node;
use core::hash::Hash;
use counter::Counter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::{FromIterator, IntoIterator};
use std::str::FromStr;
use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames, ToString};

#[derive(ToString, EnumString, EnumVariantNames, Hash, Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Continent,
    Country,
    City,
    Datacenter,
    DatacenterOwner,
    NodeProvider,
}

impl Feature {
    pub fn variants() -> Vec<Self> {
        Feature::VARIANTS
            .iter()
            .map(|f| Feature::from_str(f).unwrap())
            .collect()
    }
}

// Trait to define what features are considered for a given Type
pub trait Decentralize {
    type T: Hash + Eq + Clone;
    fn get_features(&self) -> NodeFeatures;
    fn get_feature(&self, feature: Feature) -> Self::T;
    fn check_business_rules<IterType>(candidate: IterType) -> bool
    where
        IterType: IntoIterator<Item = Self>;
}

impl Decentralize for NodeFeatures {
    type T = String;

    fn get_features(&self) -> NodeFeatures {
        let mut all_features = HashMap::new();
        for feature in &Feature::variants() {
            all_features.insert(
                feature.clone(),
                self.get(feature).unwrap_or_else(|| "unknown".to_string()).to_string(),
            );
        }
        NodeFeatures {
            feature_map: all_features,
        }
    }

    fn get_feature(&self, feature: Feature) -> String {
        self.get(&feature).unwrap_or_else(|| "unknown".to_string())
    }

    fn check_business_rules<IterType>(candidate: IterType) -> bool
    where
        IterType: IntoIterator<Item = Self>,
    {
        let mut counts: HashMap<String, i32> = HashMap::new();
        let feats = candidate.into_iter().collect::<Vec<Self>>();
        let size = feats.len();
        let max_dcs = (size / 13) + 1;
        for feat in feats {
            let dc = feat.get_feature(Feature::Datacenter);
            counts.insert(dc.clone(), counts.get(&dc).unwrap_or(&0) + 1);
        }
        for (_, v) in counts {
            if v > max_dcs as i32 {
                return false;
            }
        }
        true
    }
}

impl Decentralize for Node {
    type T = String;

    fn get_features(&self) -> NodeFeatures {
        self.features.clone()
    }

    fn get_feature(&self, feature: Feature) -> String {
        self.features.get_feature(feature)
    }

    fn check_business_rules<IterType>(candidate: IterType) -> bool
    where
        IterType: IntoIterator<Item = Self>,
    {
        let candidate_vec = candidate.into_iter().collect::<Vec<Self>>();
        candidate_vec.iter().filter(|x| x.dfinity_owned).count() >= 1
            && NodeFeatures::check_business_rules(candidate_vec.into_iter().map(|x| x.features))
    }
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct NodeFeatures {
    pub feature_map: HashMap<Feature, String>,
}

impl NodeFeatures {
    fn get(&self, feature: &Feature) -> Option<String> {
        self.feature_map.get(feature).cloned()
    }

    #[cfg(test)]
    fn new_test_feature_set(value: &str) -> Self {
        let mut result = HashMap::new();
        for feature in Feature::variants() {
            result.insert(feature, value.to_string());
        }
        NodeFeatures { feature_map: result }
    }
}

impl FromIterator<(Feature, &'static str)> for NodeFeatures {
    fn from_iter<I: IntoIterator<Item = (Feature, &'static str)>>(iter: I) -> Self {
        Self {
            feature_map: HashMap::from_iter(iter.into_iter().map(|x| (x.0, String::from(x.1)))),
        }
    }
}

impl FromIterator<(Feature, std::string::String)> for NodeFeatures {
    fn from_iter<I: IntoIterator<Item = (Feature, std::string::String)>>(iter: I) -> Self {
        Self {
            feature_map: HashMap::from_iter(iter),
        }
    }
}

pub trait Extendable
where
    Self: IntoIterator,
    Self::Item: Decentralize,
{
    fn best_extension(self, size: usize, available: &[Self::Item]) -> Option<Vec<Self::Item>>;
    fn merge<Available>(self, other: Available) -> Self
    where
        Available: IntoIterator<Item = Self::Item>;
}

impl<T> Extendable for T
where
    T: IntoIterator + FromIterator<T::Item>,
    T::Item: Decentralize + Clone + PartialEq,
{
    fn best_extension(self, size: usize, available: &[Self::Item]) -> Option<Vec<T::Item>> {
        if size == 0 {
            return Some(Vec::new());
        }
        let mut available = available.to_vec();

        let current = self.into_iter().collect::<Vec<_>>();
        let best = available
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let mut current = current.clone();
                current.push(node.clone());
                if T::Item::check_business_rules(current.clone()) {
                    Some((index, node.clone(), NakamotoScore::from(current)))
                } else {
                    None
                }
            })
            .max_by_key(|(_, _, score)| (score.total * 100.) as u64);

        best.and_then(|(index, node, _)| {
            available.swap_remove(index);
            let mut current = current.clone();
            current.push(node.clone());
            current.best_extension(size - 1, &available).map(|mut extension| {
                extension.push(node);
                extension
            })
        })
    }
    fn merge<Available>(self, other: Available) -> Self
    where
        Available: IntoIterator<Item = T::Item>,
    {
        self.into_iter().chain(other.into_iter()).collect::<Self>()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct NakamotoScore {
    scores: HashMap<String, f64>,
    total: f64,
}

impl NakamotoScore {
    /// Build a new NakamotoScore object from a vec of FeatureSet.
    pub fn from_vec_features(vec_features: Vec<NodeFeatures>) -> Self {
        let mut features_to_nodes_map = HashMap::new();

        for feature in Feature::variants() {
            features_to_nodes_map.insert(feature.to_string(), Vec::new());
        }

        // Convert a Vec<HashMap<Feature, Value>> into a Vec<HashMap<Feature,
        // Vec<Values>>
        for node_features in vec_features.iter() {
            for feature in Feature::variants() {
                let curr = features_to_nodes_map.get_mut(&feature.to_string()).unwrap();
                curr.push(node_features.get_feature(feature));
            }
        }

        let scores = features_to_nodes_map
            .iter()
            .map(|value| {
                // Turns a Vec<Features> into a Vec<(Feature, Number)>
                // where "Number" is the count of objects with the feature
                let counter: Vec<usize> = value.1.iter().collect::<Counter<_>>().iter().map(|x| *x.1).collect();

                (value.0.clone(), Self::nakamoto(&counter) as f64)
            })
            .collect::<HashMap<String, f64>>();

        // Average the totals.
        let total: f64 = scores.values().copied().sum::<f64>() / scores.len() as f64;
        NakamotoScore { scores, total }
    }

    /// Build a new NakamotoScore object from a vec of Nodes.
    pub fn from_vec_nodes(vec_nodes: Vec<Node>) -> Self {
        Self::from_vec_features(vec_nodes.into_iter().map(|n| n.features).collect())
    }

    /// The Nakamoto Coefficient represents the number of actors that would have
    /// to collude together to attack a subnet if they wanted to.
    /// This function takes a vector of numbers, where each number is the count
    /// of nodes in control of an actor.
    /// Returns a value between 1 and the total number of actors, to indicate
    /// how many top actors would be needed to break the consensus
    /// requirements
    fn nakamoto(values: &[usize]) -> usize {
        let mut values = values.to_owned();
        let total_subnet_nodes: usize = values.iter().sum();

        // The number of non-malicious actors that the consensus requires => 2f + 1
        // so at most 1/3 of the subnet nodes (actors) can be malicious (actually 1/3 -
        // 1) Source: https://dfinity.slack.com/archives/C01D7R95YJE/p1648480036670009?thread_ts=1648129258.551759&cid=C01D7R95YJE
        // > We use different thresholds in different places, depending on the security
        // we need. > Most things are fine with f+1, so >1/3rd, but some other
        // things like certification / CUPs > need to use 2f+1 (even if we only
        // assume that f can be corrupt) because we want to be more > resilient
        // against non-deterministic execution.
        let max_malicious_nodes = total_subnet_nodes / 3;

        // Reverse sort, go from actor with most to fewest repetitions.
        // The ultimate nakamoto coefficient is the number of different actors necessary
        // to reach max_malicious_actors
        values.sort_by(|a, b| b.cmp(a));

        let mut sum_actors: usize = 0;
        let mut sum_nodes: usize = 0;
        for actor_nodes in values {
            sum_actors += 1;
            sum_nodes = sum_nodes.saturating_add(actor_nodes);
            if sum_nodes > max_malicious_nodes {
                // Adding the current actor would break the consensus requirements, so stop
                // here.
                break;
            }
        }
        sum_actors
    }

    /// Get a reference to the score's total.
    pub fn total(&self) -> f64 {
        self.total
    }

    /// Get individual scores.
    pub fn scores_individual(&self) -> HashMap<String, f64> {
        self.scores.clone()
    }
}

impl<T> From<T> for NakamotoScore
where
    T: IntoIterator,
    T::Item: Decentralize,
{
    fn from(iter_feature_sets: T) -> Self {
        let vec_feature_sets = iter_feature_sets.into_iter().map(|x| x.get_features()).collect();
        NakamotoScore::from_vec_features(vec_feature_sets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Decentralize, Node, NodeFeatures};

    #[test]
    fn computes_nakamoto_scores() {
        assert_eq!(0, NakamotoScore::nakamoto(&[])); // empty vector
        assert_eq!(1, NakamotoScore::nakamoto(&[1])); // one actor controls 1 node
        assert_eq!(1, NakamotoScore::nakamoto(&[3])); // one actor controls 3 nodes
        for actors in 1..100 {
            // If 3..100 actors have 1 nodes each, then > 1/3 of the nodes needs to be
            // malicious
            assert_eq!(
                1 + (actors / 3),
                NakamotoScore::nakamoto(&std::iter::repeat(1).take(actors).collect::<Vec<usize>>())
            );
        }
        // Included above as well, but more explicit for readability: 5/13 nodes need to
        // be malicious
        assert_eq!(
            5,
            NakamotoScore::nakamoto(&std::iter::repeat(1).take(13).collect::<Vec<usize>>())
        );
        assert_eq!(1, NakamotoScore::nakamoto(&[1, 2, 3])); // one actor controls 3/6 nodes
        assert_eq!(1, NakamotoScore::nakamoto(&[2, 3, 1])); // one actor controls 3/6 nodes
        assert_eq!(1, NakamotoScore::nakamoto(&[3, 2, 1])); // one actor controls 3/6 nodes
        assert_eq!(2, NakamotoScore::nakamoto(&[1, 2, 1, 2, 1])); // two top actors control 4/7 nodes
        assert_eq!(1, NakamotoScore::nakamoto(&[1, 1, 2, 3, 5, 1])); // one top actor controls 5/13 nodes
        assert_eq!(2, NakamotoScore::nakamoto(&[1, 1, 2, 3, 5, 1, 2])); // two top actors control 8/15 nodes
    }

    #[test]
    fn score_from_features() {
        let features = vec![NodeFeatures::new_test_feature_set("foo")];
        let score = NakamotoScore::from_vec_features(features);

        let score_expected = NakamotoScore {
            scores: HashMap::from([
                (Feature::City.to_string(), 1.),
                (Feature::Country.to_string(), 1.),
                (Feature::Continent.to_string(), 1.),
                (Feature::DatacenterOwner.to_string(), 1.),
                (Feature::NodeProvider.to_string(), 1.),
                (Feature::Datacenter.to_string(), 1.),
            ]),
            total: 1.,
        };
        assert_eq!(score, score_expected);
    }

    #[test]
    fn test_business_rules_pass() {
        // If there is exactly one DFINITY-owned node in a small subnet ==> pass
        let nodes = vec![
            Node::new_test_node(0, NodeFeatures::new_test_feature_set("foo"), true),
            Node::new_test_node(0, NodeFeatures::new_test_feature_set("bar"), false),
        ];
        assert!(Node::check_business_rules(nodes));
    }

    #[test]
    fn test_business_rules_fail() {
        // If there are no DFINITY-owned node in a small subnet ==> fail
        let nodes = vec![
            Node::new_test_node(0, NodeFeatures::new_test_feature_set("foo"), false),
            Node::new_test_node(0, NodeFeatures::new_test_feature_set("bar"), false),
        ];
        assert!(!Node::check_business_rules(nodes));
    }

    #[test]
    fn extend_feature_set_group() {
        let nodes_initial = vec![Node::new_test_node(0, NodeFeatures::new_test_feature_set("foo"), true)];
        let nodes_available = vec![Node::new_test_node(0, NodeFeatures::new_test_feature_set("bar"), false)];

        let extension = nodes_initial.best_extension(1, &nodes_available).unwrap();
        assert_eq!(extension, nodes_available);
    }
}
