use crate::network::Node;
use ahash::{AHashMap, AHasher};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hasher;
use std::iter::{FromIterator, IntoIterator};

use ic_management_types::NodeFeature;

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct NodeFeatures {
    pub feature_map: BTreeMap<NodeFeature, String>,
}

impl NodeFeatures {
    pub fn get(&self, feature: &NodeFeature) -> Option<String> {
        self.feature_map.get(feature).cloned()
    }

    #[cfg(test)]
    fn new_test_feature_set(value: &str) -> Self {
        let mut result = BTreeMap::new();
        for feature in NodeFeature::variants() {
            result.insert(feature, value.to_string());
        }
        NodeFeatures { feature_map: result }
    }

    #[cfg(test)]
    fn with_feature_value(&self, feature: &NodeFeature, value: &str) -> Self {
        let mut feature_map = self.feature_map.clone();
        feature_map.insert(feature.clone(), value.to_string());
        NodeFeatures { feature_map }
    }
}

impl FromIterator<(NodeFeature, &'static str)> for NodeFeatures {
    fn from_iter<I: IntoIterator<Item = (NodeFeature, &'static str)>>(iter: I) -> Self {
        Self {
            feature_map: BTreeMap::from_iter(iter.into_iter().map(|x| (x.0, String::from(x.1)))),
        }
    }
}

impl FromIterator<(NodeFeature, std::string::String)> for NodeFeatures {
    fn from_iter<I: IntoIterator<Item = (NodeFeature, std::string::String)>>(iter: I) -> Self {
        Self {
            feature_map: BTreeMap::from_iter(iter),
        }
    }
}

// A thread-local memoization cache of NakamotoScores
thread_local! {
    pub static NAKAMOTOSCORE_CACHE: RefCell<AHashMap<u64, NakamotoScore>> = RefCell::new(AHashMap::new());
    pub static MEMOIZE_REQ: RefCell<u32> = RefCell::new(0);
    pub static MEMOIZE_HIT: RefCell<u32> = RefCell::new(0);
    pub static MEMOIZE_HIT_RATES: RefCell<VecDeque<u32>> = RefCell::new(VecDeque::new());
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
/// This struct keeps the Nakamoto coefficients for each feature that we track
/// for the IC nodes https://crosstower.com/resources/education/nakamoto-coefficient/
/// For instance: [NodeFeature::NodeProvider], [NodeFeature::DataCenter], etc...
/// For a complete reference check [NodeFeature]
pub struct NakamotoScore {
    coefficients: BTreeMap<NodeFeature, f64>,
    value_counts: BTreeMap<NodeFeature, Vec<(String, usize)>>,
    controlled_nodes: BTreeMap<NodeFeature, usize>,
    avg_linear: f64,

    /// This field needs to be optional in case we get a -inf result because of
    /// an empty subnet.
    /// In such a case, serialization would fail because serde does not support
    /// serialization of this value, and serializes it as a null JSON value.
    /// When trying to deserialize it as a f64, we would get a
    /// deserialization error.
    ///
    /// See serde bug for tracking here
    /// https://github.com/serde-rs/json/issues/202
    ///
    /// Also, see return value given to avg_log2 of
    /// Self.new_from_slice_node_features for implementation detail
    avg_log2: Option<f64>,
    min: f64,
}

impl NakamotoScore {
    /// Build a new NakamotoScore object from a slice of [NodeFeatures].
    pub fn new_from_slice_node_features(slice_node_features: &[NodeFeatures]) -> Self {
        let mut features_to_nodes_map = BTreeMap::new();

        for feature in NodeFeature::variants() {
            features_to_nodes_map.insert(feature, Vec::new());
        }

        // Convert a Vec<BTreeMap<NodeFeature, Value>> into a Vec<BTreeMap<NodeFeature,
        // Vec<Values>>
        for node_features in slice_node_features.iter() {
            for feature in NodeFeature::variants() {
                let curr = features_to_nodes_map.get_mut(&feature).unwrap();
                curr.push(node_features.get(&feature));
            }
        }

        let nakamoto_calc = features_to_nodes_map.iter().map(|value| {
            // Turns a Vec<Features> into a Vec<(NodeFeature, Number)>
            // where "Number" is the count of objects with the feature
            let counters: Vec<(String, usize)> = value
                .1
                .iter()
                // AHashMap is a very fast HashMap implementation https://github.com/tkaitchuck/aHash
                // We use it here to count the number of times each value appears in the input vector
                // Doing this with a fold instead of using https://github.com/coriolinus/counter-rs is faster
                .fold(AHashMap::new(), |mut acc: AHashMap<String, u32>, s| {
                    if let Some(s) = s {
                        acc.entry(s.to_string()).and_modify(|v| *v += 1).or_insert(1);
                    }
                    acc
                })
                .into_iter()
                .map(|(feat, cnt)| (feat, cnt as usize))
                .collect::<Vec<_>>();

            // We only care about the counts to calculate the Nakamoto Coefficient, so we
            // discard the feature names
            let only_counter = counters.iter().map(|(_feat, cnt)| *cnt).collect::<Vec<_>>();
            // But for deeper understanding (logging and debugging) we also keep track of
            // all strings and their counts
            let value_counts = counters.into_iter().sorted_by_key(|(_feat, cnt)| -(*cnt as isize)).collect::<Vec<_>>();

            (value.0.clone(), Self::nakamoto(&only_counter), value_counts)
        });

        let scores = nakamoto_calc
            .clone()
            .map(|(f, n, _)| (f, n.0 as f64))
            .collect::<BTreeMap<NodeFeature, f64>>();

        let controlled_nodes = nakamoto_calc.clone().map(|(f, n, _)| (f, n.1)).collect::<BTreeMap<NodeFeature, usize>>();

        let value_counts = nakamoto_calc.map(|(f, _, value_counts)| (f, value_counts)).collect();

        NakamotoScore {
            coefficients: scores.clone(),
            value_counts,
            controlled_nodes,
            avg_linear: scores.values().sum::<f64>() / scores.len() as f64,
            avg_log2: {
                // See struct definition for this field for the exlpanation of this
                // condition
                if scores.values().all(|&v| v != 0 as f64) {
                    Some(scores.values().map(|x| x.log2()).sum::<f64>() / scores.len() as f64)
                } else {
                    None
                }
            },
            min: scores
                .values()
                .map(|x| if x.is_finite() { *x } else { 0. })
                .fold(1.0 / 0.0, |acc, x| if x < acc { x } else { acc }),
        }
    }

    /// Build a new NakamotoScore object from a slice of [Node]s.
    pub fn new_from_nodes(nodes: &[Node]) -> Self {
        let mut memoize_key = AHasher::default();
        for node in nodes.iter().sorted_by_cached_key(|n| n.id) {
            for byte in node.id.0.as_slice() {
                memoize_key.write_u8(*byte);
            }
        }
        let memoize_key = memoize_key.finish();
        NAKAMOTOSCORE_CACHE.with(|memoize_cache| {
            MEMOIZE_REQ.with(|memoize_req| {
                MEMOIZE_HIT.with(|memoize_hit| {
                    MEMOIZE_HIT_RATES.with(|memoize_hit_rates| {
                        *memoize_req.borrow_mut() += 1;
                        let mut memoize_cache = memoize_cache.borrow_mut();
                        match memoize_cache.get(&memoize_key) {
                            Some(score) => {
                                *memoize_hit.borrow_mut() += 1;
                                if memoize_req.borrow().checked_rem(10000) == Some(0) {
                                    let memoize_hit_rate = *memoize_hit.borrow() * 100 / *memoize_req.borrow();
                                    if memoize_hit_rate > 0 {
                                        memoize_hit_rates.borrow_mut().push_front(memoize_hit_rate);
                                        if memoize_hit_rates.borrow().len() > 50 {
                                            memoize_hit_rates.borrow_mut().pop_back();
                                        }
                                    }
                                    println!("Memoize hit rate: {}%", memoize_hit_rate);
                                    println!("Memoize recent hit rates: {:?}", memoize_hit_rates.borrow());
                                    *memoize_req.borrow_mut() = 0;
                                    *memoize_hit.borrow_mut() = 0;
                                }
                                score.clone()
                            }
                            None => {
                                let score = Self::new_from_slice_node_features(&nodes.iter().map(|n| n.features.clone()).collect::<Vec<_>>());
                                memoize_cache.insert(memoize_key, score.clone());
                                score
                            }
                        }
                    })
                })
            })
        })
    }

    /// The Nakamoto Coefficient represents the number of actors that would have
    /// to collude together to attack a subnet if they wanted to.
    /// This function takes a vector of numbers, where each number is the count
    /// of nodes in control of an actor.
    /// Returns:
    /// 1) a value between 1 and the total number of actors, to indicate
    ///    how many top actors would be needed to break the consensus
    ///    requirements
    /// 2) the number of nodes that the top actors control
    fn nakamoto(values: &[usize]) -> (usize, usize) {
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
        (sum_actors, sum_nodes)
    }

    /// An average of the linear nakamoto scores over all features
    pub fn score_avg_linear(&self) -> f64 {
        self.avg_linear
    }

    /// An average of the log2 nakamoto scores over all features
    pub fn score_avg_log2(&self) -> Option<f64> {
        self.avg_log2
    }

    /// A minimum Nakamoto score over all features
    pub fn score_min(&self) -> f64 {
        self.min
    }

    /// Get a Map with all the features and the corresponding Nakamoto score
    pub fn scores_individual(&self) -> BTreeMap<NodeFeature, f64> {
        self.coefficients.clone()
    }

    /// Get the Nakamoto score for a single feature
    pub fn score_feature(&self, feature: &NodeFeature) -> Option<f64> {
        self.coefficients.get(feature).copied()
    }

    /// Get the max count for the given feature - this is the number of
    /// repetitions of the most common value
    pub fn feature_value_counts_max(&self, feature: &NodeFeature) -> Option<(String, usize)> {
        let counts = self.feature_value_counts(feature);
        counts.first().cloned()
    }

    /// Get the value count for the given feature
    pub fn feature_value_counts(&self, feature: &NodeFeature) -> Vec<(String, usize)> {
        match self.value_counts.get(feature) {
            Some(value_counts) => value_counts.to_vec(),
            None => vec![],
        }
    }

    /// Critical features are Node Provider and Country.
    /// Count (upper bound of) the number of nodes controlled by the top actors
    /// in each of these features.
    /// - Top Node Providers control 5 nodes
    /// - Top Countries control 7 nodes
    /// In that case we would return (5, 7)
    pub fn critical_features_num_nodes(&self) -> Vec<usize> {
        [NodeFeature::NodeProvider, NodeFeature::Country]
            .iter()
            .map(|feat| self.controlled_nodes.get(feat).cloned().unwrap_or_default())
            .collect()
    }

    /// Number of unique actors for the critical features.
    /// E.g. if there are 5 unique (different) NPs in a subnet ==> return 5
    pub fn critical_features_unique_actors(&self) -> Vec<usize> {
        [NodeFeature::NodeProvider, NodeFeature::Country]
            .iter()
            .map(|feat| self.feature_value_counts(feat).len())
            .collect()
    }

    /// Return the number of nodes that the top actors control
    pub fn controlled_nodes(&self, feature: &NodeFeature) -> Option<usize> {
        self.controlled_nodes.get(feature).copied()
    }
}

impl Ord for NakamotoScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("partial_cmp failed")
    }
}

impl PartialOrd for NakamotoScore {
    /// By default, the higher value will take the precedence
    #[allow(clippy::non_canonical_partial_ord_impl)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Prefer higher score across all features
        let mut cmp = self.score_min().partial_cmp(&other.score_min());

        if cmp != Some(Ordering::Equal) {
            return cmp;
        }

        // Then try to increase the log2 avg
        cmp = self.score_avg_log2().partial_cmp(&other.score_avg_log2());

        if cmp != Some(Ordering::Equal) {
            return cmp;
        }

        // Try to pick the candidate that *reduces* the number of nodes
        // controlled by the top actors
        cmp = other.critical_features_num_nodes().partial_cmp(&self.critical_features_num_nodes());

        if cmp != Some(Ordering::Equal) {
            return cmp;
        }

        // Compare the number of unique actors for the critical features
        // E.g. self has 5 NPs and other has 4 NPs ==> prefer self
        cmp = self
            .critical_features_unique_actors()
            .partial_cmp(&other.critical_features_unique_actors());

        if cmp != Some(Ordering::Equal) {
            return cmp;
        }

        // Compare the count of below-average coefficients
        // and prefer candidates that decrease the number of low-value coefficients
        let c1 = self.coefficients.values().filter(|c| **c < 3.0).count();
        let c2 = other.coefficients.values().filter(|c| **c < 3.0).count();
        cmp = c2.partial_cmp(&c1);

        if cmp != Some(Ordering::Equal) {
            return cmp;
        }

        // If the worst feature is the same for both candidates
        // => prefer candidates that maximizes all features
        for feature in NodeFeature::variants() {
            let c1 = self.coefficients.get(&feature).unwrap_or(&1.0);
            let c2 = other.coefficients.get(&feature).unwrap_or(&1.0);
            if *c1 < 3.0 || *c2 < 3.0 {
                // Ensure that the new candidate does not decrease the critical features (that
                // are below the average)
                cmp = c2.partial_cmp(c1);

                if cmp != Some(Ordering::Equal) {
                    return cmp;
                }
            }
        }

        // And finally try to increase the linear average
        self.score_avg_linear().partial_cmp(&other.score_avg_linear())
    }
}

impl PartialEq for NakamotoScore {
    fn eq(&self, other: &Self) -> bool {
        self.coefficients == other.coefficients && self.controlled_nodes == other.controlled_nodes
    }
}

impl Eq for NakamotoScore {}

impl Display for NakamotoScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let avg_log2_str = match self.avg_log2 {
            Some(v) => format!("{:0.2}", v),
            None => "undefined".to_string(),
        };
        write!(
            f,
            "NakamotoScore: min {:0.2} avg log2 {} #crit nodes {:?} # crit uniq {:?} #crit coeff {} avg linear {:0.2}",
            self.min,
            avg_log2_str,
            self.critical_features_num_nodes(),
            self.critical_features_unique_actors(),
            self.coefficients.values().filter(|c| **c < 3.0).count(),
            self.avg_linear,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::network::{DecentralizedSubnet, NetworkHealRequest, NetworkHealSubnets, SubnetChangeRequest};
    use ic_base_types::PrincipalId;
    use itertools::Itertools;
    use regex::Regex;

    use super::*;
    use super::{Node, NodeFeatures};

    #[test]
    fn computes_nakamoto_scores() {
        assert_eq!((0, 0), NakamotoScore::nakamoto(&[])); // empty vector
        assert_eq!((1, 1), NakamotoScore::nakamoto(&[1])); // one actor controls 1 node
        assert_eq!((1, 3), NakamotoScore::nakamoto(&[3])); // one actor controls 3 nodes
        for actors in 1..100 {
            // If 3..100 actors have 1 nodes each, then > 1/3 of the nodes needs to be
            // malicious
            assert_eq!(
                (1 + (actors / 3), 1 + (actors / 3)),
                NakamotoScore::nakamoto(&std::iter::repeat(1).take(actors).collect::<Vec<usize>>())
            );
        }
        // Included above as well, but more explicit for readability: 5/13 nodes need to
        // be malicious
        assert_eq!((5, 5), NakamotoScore::nakamoto(&std::iter::repeat(1).take(13).collect::<Vec<usize>>()));
        assert_eq!((1, 3), NakamotoScore::nakamoto(&[1, 2, 3])); // one actor controls 3/6 nodes
        assert_eq!((1, 3), NakamotoScore::nakamoto(&[2, 3, 1])); // one actor controls 3/6 nodes
        assert_eq!((1, 3), NakamotoScore::nakamoto(&[3, 2, 1])); // one actor controls 3/6 nodes
        assert_eq!((2, 4), NakamotoScore::nakamoto(&[1, 2, 1, 2, 1])); // two top actors control 4/7 nodes
        assert_eq!((1, 5), NakamotoScore::nakamoto(&[1, 1, 2, 3, 5, 1])); // one top actor controls 5/13 nodes
        assert_eq!((2, 8), NakamotoScore::nakamoto(&[1, 1, 2, 3, 5, 1, 2])); // two top actors control 8/15 nodes
    }

    #[test]
    fn score_from_features() {
        let features = vec![NodeFeatures::new_test_feature_set("foo")];
        let score = NakamotoScore::new_from_slice_node_features(&features);

        let score_expected = NakamotoScore {
            coefficients: BTreeMap::from([
                (NodeFeature::City, 1.),
                (NodeFeature::Country, 1.),
                (NodeFeature::Continent, 1.),
                (NodeFeature::DataCenterOwner, 1.),
                (NodeFeature::NodeProvider, 1.),
                (NodeFeature::DataCenter, 1.),
            ]),
            value_counts: BTreeMap::new(),
            controlled_nodes: BTreeMap::from([
                (NodeFeature::City, 1),
                (NodeFeature::Country, 1),
                (NodeFeature::Continent, 1),
                (NodeFeature::DataCenterOwner, 1),
                (NodeFeature::NodeProvider, 1),
                (NodeFeature::DataCenter, 1),
            ]),
            avg_linear: 1.,
            avg_log2: Some(0.),
            min: 1.,
        };
        assert_eq!(score, score_expected);
    }

    /// Generate a new Vec<Node> of len num_nodes, out of which
    /// num_dfinity_nodes are DFINITY-owned
    fn new_test_nodes(feat_prefix: &str, num_nodes: usize, num_dfinity_nodes: usize) -> Vec<Node> {
        let mut subnet_nodes = Vec::new();
        for i in 0..num_nodes {
            let dfinity_owned = i < num_dfinity_nodes;
            let node_features = NodeFeatures::new_test_feature_set(&format!("{} {}", feat_prefix, i));
            let node = Node::new_test_node(i as u64, node_features, dfinity_owned, true);
            subnet_nodes.push(node);
        }
        subnet_nodes
    }

    /// Generate a new Vec<Node> and override some feature values
    fn new_test_nodes_with_overrides(
        feat_prefix: &str,
        node_number_start: usize,
        num_nodes: usize,
        num_dfinity_nodes: usize,
        feature_to_override: (&NodeFeature, &[&str]),
    ) -> Vec<Node> {
        let mut subnet_nodes = Vec::new();
        for i in 0..num_nodes {
            let dfinity_owned = i < num_dfinity_nodes;
            let (override_feature, override_val) = feature_to_override;
            let node_features = match override_val.get(i) {
                Some(override_val) => {
                    NodeFeatures::new_test_feature_set(&format!("{} {}", feat_prefix, i)).with_feature_value(override_feature, override_val)
                }
                None => NodeFeatures::new_test_feature_set(&format!("feat {}", i)),
            };
            let node = Node::new_test_node((node_number_start + i) as u64, node_features, dfinity_owned, true);
            subnet_nodes.push(node);
        }
        subnet_nodes
    }

    /// Generate a new test subnet with num_nodes, out of which
    /// num_dfinity_nodes are DFINITY-owned
    fn new_test_subnet(subnet_num: u64, num_nodes: usize, num_dfinity_nodes: usize) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: PrincipalId::new_subnet_test_id(subnet_num),
            nodes: new_test_nodes("feat", num_nodes, num_dfinity_nodes),
            removed_nodes: Vec::new(),
            min_nakamoto_coefficients: None,
            comment: None,
            run_log: Vec::new(),
        }
    }

    /// Generate a new test subnet with feature overrides
    fn new_test_subnet_with_overrides(
        subnet_num: u64,
        node_number_start: usize,
        num_nodes: usize,
        num_dfinity_nodes: usize,
        feature_to_override: (&NodeFeature, &[&str]),
    ) -> DecentralizedSubnet {
        DecentralizedSubnet {
            id: PrincipalId::new_subnet_test_id(subnet_num),
            nodes: new_test_nodes_with_overrides("feat", node_number_start, num_nodes, num_dfinity_nodes, feature_to_override),
            removed_nodes: Vec::new(),
            min_nakamoto_coefficients: None,
            comment: None,
            run_log: Vec::new(),
        }
    }

    #[test]
    fn test_business_rules_pass() {
        // If there is exactly one DFINITY-owned node in a subnet ==> pass
        new_test_subnet(0, 7, 1).check_business_rules().unwrap();
        new_test_subnet(0, 12, 1).check_business_rules().unwrap();
        new_test_subnet(0, 13, 1).check_business_rules().unwrap();
        new_test_subnet(0, 13, 2).check_business_rules().unwrap();
        new_test_subnet(0, 14, 1).check_business_rules().unwrap();
        new_test_subnet(0, 25, 1).check_business_rules().unwrap();
        new_test_subnet(0, 25, 2).check_business_rules().unwrap();
        new_test_subnet(0, 26, 1).check_business_rules().unwrap();
        new_test_subnet(0, 26, 2).check_business_rules().unwrap();
        new_test_subnet(0, 27, 2).check_business_rules().unwrap();
        new_test_subnet(0, 38, 2).check_business_rules().unwrap();
        new_test_subnet(0, 38, 3).check_business_rules().unwrap();
        new_test_subnet(0, 39, 3).check_business_rules().unwrap();
        new_test_subnet(0, 40, 3).check_business_rules().unwrap();
        new_test_subnet(0, 51, 3).check_business_rules().unwrap();
        new_test_subnet(0, 51, 4).check_business_rules().unwrap();
        new_test_subnet(0, 52, 4).check_business_rules().unwrap();
        new_test_subnet(0, 53, 4).check_business_rules().unwrap();
    }

    #[test]
    fn test_business_rules_fail() {
        // If there are no DFINITY-owned node in a small subnet ==> fail with an
        // expected error message
        assert_eq!(
            new_test_subnet(0, 2, 0).check_business_rules().unwrap(),
            (1000, vec!["Subnet should have 1 DFINITY-owned nodes, got 0".to_string()])
        );
    }

    #[test]
    fn extend_feature_set_group() {
        let subnet_initial = new_test_subnet(0, 12, 1);
        let nodes_initial = subnet_initial.nodes.clone();
        let nodes_available = new_test_nodes("spare", 1, 0);

        let extended_subnet = subnet_initial.subnet_with_more_nodes(1, &nodes_available).unwrap();
        assert_eq!(
            extended_subnet.nodes,
            nodes_initial.iter().chain(nodes_available.iter()).cloned().collect::<Vec<_>>()
        );
    }

    #[test]
    fn subnet_usa_dominance() {
        let subnet_initial = new_test_subnet_with_overrides(
            0,
            0,
            13,
            1,
            (
                &NodeFeature::Country,
                &["US", "US", "US", "US", "US", "US", "US", "US", "US", "CH", "BE", "SG", "SI"],
            ),
        );
        assert_eq!(
            subnet_initial.check_business_rules().unwrap(),
            (
                1000,
                vec!["NodeFeature 'country' controls 9 of nodes, which is > 8 (2/3 of all) nodes".to_string()]
            )
        );
        let nodes_available = new_test_nodes_with_overrides("spare", 13, 3, 0, (&NodeFeature::Country, &["US", "RO", "JP"]));

        println!(
            "initial {} Countries {:?}",
            subnet_initial,
            subnet_initial
                .nodes
                .iter()
                .map(|n| n.get_feature(&NodeFeature::Country))
                .collect::<Vec<_>>()
        );

        let subnet_change_req = SubnetChangeRequest::new(subnet_initial, nodes_available, Vec::new(), Vec::new(), None);
        let subnet_change = subnet_change_req.optimize(2, &vec![]).unwrap();
        for log in subnet_change.after().run_log.iter() {
            println!("{}", log);
        }
        let optimized_subnet = subnet_change.after();

        let countries_after = optimized_subnet
            .nodes
            .iter()
            .map(|n| n.get_feature(&NodeFeature::Country))
            .sorted()
            .collect::<Vec<_>>();

        println!("optimized {} Countries {:?}", optimized_subnet, countries_after);
        assert_eq!(optimized_subnet.nakamoto_score().score_min(), 1.);

        // Two US nodes were removed
        assert_eq!(
            countries_after,
            vec!["BE", "CH", "JP", "RO", "SG", "SI", "US", "US", "US", "US", "US", "US", "US"]
        );
    }

    #[test]
    fn subnet_optimize_node_providers() {
        // NP2 owns 3 from 7 nodes, so it can halt the subnet
        let subnet_initial = new_test_subnet_with_overrides(
            0,
            0,
            7,
            1,
            (&NodeFeature::NodeProvider, &["NP1", "NP2", "NP2", "NP2", "NP3", "NP4", "NP5"]),
        );
        assert_eq!(
            subnet_initial.check_business_rules().unwrap(),
            (10000, vec!["A single Node Provider can halt the subnet".to_string()])
        );
        let nodes_available = new_test_nodes_with_overrides("spare", 7, 2, 0, (&NodeFeature::NodeProvider, &["NP6", "NP7"]));

        println!(
            "initial {} NPs {:?}",
            subnet_initial,
            subnet_initial
                .nodes
                .iter()
                .map(|n| n.get_feature(&NodeFeature::NodeProvider))
                .collect::<Vec<_>>()
        );

        let subnet_change_req = SubnetChangeRequest::new(subnet_initial, nodes_available, Vec::new(), Vec::new(), None);
        let subnet_change = subnet_change_req.optimize(2, &vec![]).unwrap();
        println!("Replacement run log:");
        for line in subnet_change.after().run_log.iter() {
            println!("{}", line);
        }
        let optimized_subnet = subnet_change.after();

        let nps_after = optimized_subnet
            .nodes
            .iter()
            .map(|n| n.get_feature(&NodeFeature::NodeProvider))
            .sorted()
            .collect::<Vec<_>>();

        println!("optimized {} NPs {:?}", optimized_subnet, nps_after);
        assert_eq!(optimized_subnet.nakamoto_score().score_min(), 3.);

        // Check that the selected nodes are providing the maximum uniformness (use all
        // NPs)
        assert_eq!(nps_after, vec!["NP1", "NP2", "NP3", "NP4", "NP5", "NP6", "NP7"]);
    }

    #[test]
    fn subnet_optimize_prefer_non_dfinity() {
        let subnet_initial = new_test_subnet_with_overrides(
            0,
            0,
            7,
            1,
            (&NodeFeature::NodeProvider, &["NP1", "NP2", "NP2", "NP3", "NP4", "NP4", "NP5"]),
        );
        assert_eq!(subnet_initial.check_business_rules().unwrap(), (0, vec![]));

        // There are 2 spare nodes, but both are DFINITY
        let nodes_available = new_test_nodes_with_overrides("spare", 7, 2, 2, (&NodeFeature::NodeProvider, &["NP6", "NP7"]));

        println!(
            "initial {} NPs {:?}",
            subnet_initial,
            subnet_initial
                .nodes
                .iter()
                .map(|n| n.get_feature(&NodeFeature::NodeProvider))
                .collect::<Vec<_>>()
        );

        let subnet_change_req = SubnetChangeRequest::new(subnet_initial, nodes_available, Vec::new(), Vec::new(), None);
        let subnet_change = subnet_change_req.optimize(2, &vec![]).unwrap();

        println!("Replacement run log:");
        for line in subnet_change.after().run_log.iter() {
            println!("{}", line);
        }

        let optimized_subnet = subnet_change.after();

        let nps_after = optimized_subnet
            .nodes
            .iter()
            .map(|n| n.get_feature(&NodeFeature::NodeProvider))
            .sorted()
            .collect::<Vec<_>>();

        println!("optimized {} NPs {:?}", optimized_subnet, nps_after);
        assert_eq!(optimized_subnet.nakamoto_score().score_min(), 2.);
        // There is still only one DFINITY-owned node in the subnet
        assert_eq!(1, optimized_subnet.nodes.iter().map(|n| n.dfinity_owned as u32).sum::<u32>());
    }

    #[test]
    fn subnet_uzr34_extend() {
        // Read the subnet snapshot from a file
        let subnet_all =
            serde_json::from_str::<ic_management_types::Subnet>(include_str!("../../test_data/subnet-uzr34.json")).expect("failed to read test data");
        // Convert the subnet snapshot to the "Subnet" struct
        let subnet_all: DecentralizedSubnet = DecentralizedSubnet::from(subnet_all);
        let re_unhealthy_nodes = Regex::new(r"^(gp7wd|e4ysi|qhz4y|2fbvp)-.+$").unwrap();
        let subnet_healthy: DecentralizedSubnet = DecentralizedSubnet {
            id: subnet_all.id,
            nodes: subnet_all
                .nodes
                .iter()
                .filter(|n| !re_unhealthy_nodes.is_match(&n.id.to_string()))
                .cloned()
                .collect(),
            removed_nodes: Vec::new(),
            min_nakamoto_coefficients: None,
            comment: None,
            run_log: Vec::new(),
        };

        let available_nodes = serde_json::from_str::<Vec<ic_management_types::Node>>(include_str!("../../test_data/available-nodes.json"))
            .expect("failed to read test data");

        let available_nodes = available_nodes
            .iter()
            .sorted_by(|a, b| a.principal.cmp(&b.principal))
            .filter(|n| n.subnet_id.is_none() && n.proposal.is_none())
            .map(Node::from)
            .map(|n| Node { decentralized: true, ..n })
            .collect::<Vec<_>>();

        subnet_healthy.check_business_rules().expect("Check business rules failed");

        println!("Initial subnet {}", subnet_healthy);
        println!("Check business rules: {:?}", subnet_healthy.check_business_rules());
        let nakamoto_score_before = subnet_healthy.nakamoto_score();
        println!("NakamotoScore before {}", nakamoto_score_before);

        let extended_subnet = subnet_healthy.subnet_with_more_nodes(4, &available_nodes).unwrap();
        println!("{}", extended_subnet);
        let nakamoto_score_after = extended_subnet.nakamoto_score();
        println!("NakamotoScore after {}", nakamoto_score_after);

        // Check against the close-to-optimal values obtained by data analysis
        assert!(nakamoto_score_after.score_min() >= 1.0);
        assert!(nakamoto_score_after.critical_features_num_nodes()[0] <= 25);
        assert!(nakamoto_score_after.score_avg_linear() >= 3.0);
        assert!(nakamoto_score_after.score_avg_log2() >= Some(1.32));
    }

    #[test]
    fn test_extend_empty_subnet() {
        let available_nodes = (0..20)
            .map(|i| Node::new_test_node(i, NodeFeatures::new_test_feature_set(&format!("foo{i}")), i % 10 == 0, true))
            .collect::<Vec<_>>();
        let empty_subnet = DecentralizedSubnet::default();

        let want_subnet_size = 13;
        let new_subnet_result = empty_subnet.subnet_with_more_nodes(want_subnet_size, &available_nodes);
        assert!(new_subnet_result.is_ok(), "error: {:?}", new_subnet_result.err());

        let new_subnet = new_subnet_result.unwrap();
        assert_eq!(new_subnet.nodes.len(), want_subnet_size)
    }

    #[test]
    fn test_european_subnet_european_nodes_good() {
        let subnet_initial = new_test_subnet_with_overrides(
            0,
            0,
            7,
            1,
            (
                &NodeFeature::Continent,
                &["Europe", "Europe", "Europe", "Europe", "Europe", "Europe", "Europe"],
            ),
        )
        .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        assert_eq!(subnet_initial.check_business_rules().unwrap(), (0, vec![]));
    }

    #[test]
    fn test_european_subnet_european_nodes_bad_1() {
        let subnet_mix = new_test_subnet_with_overrides(
            1,
            0,
            7,
            1,
            (
                &NodeFeature::Continent,
                &["Europe", "Asia", "Europe", "Europe", "Europe", "Europe", "Europe"],
            ),
        )
        .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        assert_eq!(
            subnet_mix.check_business_rules().unwrap(),
            (1000, vec!["European subnet has 1 non-European node(s)".to_string()])
        );
    }
    #[test]
    fn test_european_subnet_european_nodes_bad_2() {
        let subnet_mix = new_test_subnet_with_overrides(
            1,
            0,
            7,
            1,
            (
                &NodeFeature::Continent,
                &["Europe", "Asia", "America", "Australia", "Europe", "Africa", "South America"],
            ),
        )
        .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        assert_eq!(
            subnet_mix.check_business_rules().unwrap(),
            (5000, vec!["European subnet has 5 non-European node(s)".to_string()])
        );
    }

    #[test]
    fn test_network_heal_subnets_ord() {
        let not_important_small = new_test_subnet(0, 13, 0)
            .with_subnet_id(PrincipalId::from_str("k44fs-gm4pv-afozh-rs7zw-cg32n-u7xov-xqyx3-2pw5q-eucnu-cosd4-uqe").unwrap());
        let not_important_small = NetworkHealSubnets {
            name: String::from("App 20"),
            decentralized_subnet: not_important_small,
            unhealthy_nodes: vec![],
        };

        let not_important_large = new_test_subnet(0, 28, 0)
            .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        let not_important_large = NetworkHealSubnets {
            name: String::from("European"),
            decentralized_subnet: not_important_large,
            unhealthy_nodes: vec![],
        };

        let important =
            serde_json::from_str::<ic_management_types::Subnet>(include_str!("../../test_data/subnet-uzr34.json")).expect("failed to read test data");
        let important = NetworkHealSubnets {
            name: important.metadata.name.clone(),
            decentralized_subnet: DecentralizedSubnet::from(important),
            unhealthy_nodes: vec![],
        };

        let unordered = vec![not_important_small.clone(), important.clone(), not_important_large.clone()];
        let healing_order = unordered.clone().into_iter().sorted_by(|a, b| a.cmp(b).reverse()).collect_vec();

        assert_eq!(vec![important, not_important_large, not_important_small], healing_order);
    }

    #[test]
    fn test_network_heal() {
        let nodes_available = new_test_nodes("spare", 10, 2);
        let nodes_available_principals = nodes_available.iter().map(|n| n.id).collect_vec();

        let important =
            serde_json::from_str::<ic_management_types::Subnet>(include_str!("../../test_data/subnet-uzr34.json")).expect("failed to read test data");
        let important_decentralized = DecentralizedSubnet::from(important.clone());
        let important_unhealthy_principals = vec![
            PrincipalId::from_str("e4ysi-xp4fs-5ckcv-7e76q-edydw-ak6le-2acyt-k7udb-lj2vo-fqhhx-vqe").unwrap(),
            PrincipalId::from_str("aefqq-d7ldg-ljk5s-cmnxk-qqu7c-tw52l-74g3m-xxl5d-ag4ia-dxubz-wae").unwrap(),
        ];
        let unhealthy_nodes = important_decentralized
            .nodes
            .clone()
            .into_iter()
            .filter(|n| important_unhealthy_principals.contains(&n.id))
            .collect_vec();
        let important = NetworkHealSubnets {
            name: important.metadata.name.clone(),
            decentralized_subnet: important_decentralized,
            unhealthy_nodes: unhealthy_nodes.clone(),
        };

        let network_heal_response = NetworkHealRequest::new(vec![important])
            .heal_and_optimize(nodes_available.clone(), None)
            .unwrap();

        let result = network_heal_response.first().unwrap().clone();

        assert_eq!(important_unhealthy_principals, result.removed.clone());

        assert_eq!(important_unhealthy_principals.len(), result.added.len());

        result.added.iter().for_each(|n| assert!(nodes_available_principals.contains(n)));
    }
}
