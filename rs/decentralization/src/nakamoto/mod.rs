use ahash::{AHashMap, AHasher};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hasher;
use std::iter::IntoIterator;

use ic_management_types::{Node, NodeFeature, NodeFeatures};

// A thread-local memoization cache of NakamotoScores
thread_local! {
    pub static NAKAMOTOSCORE_CACHE: RefCell<AHashMap<u64, NakamotoScore>> = RefCell::new(AHashMap::new());
    pub static MEMOIZE_REQ: RefCell<u32> = const { RefCell::new(0) };
    pub static MEMOIZE_HIT: RefCell<u32> = const { RefCell::new(0) };
    pub static MEMOIZE_HIT_RATES: RefCell<VecDeque<u32>> = const { RefCell::new(VecDeque::new()) };
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
/// This struct keeps the Nakamoto coefficients for each feature that we track
/// for the IC nodes https://crosstower.com/resources/education/nakamoto-coefficient/
/// For instance: [NodeFeature::NodeProvider], [NodeFeature::DataCenter], etc...
/// For a complete reference check [NodeFeature]
pub struct NakamotoScore {
    coefficients: IndexMap<NodeFeature, f64>,
    value_counts: IndexMap<NodeFeature, Vec<(String, usize)>>,
    controlled_nodes: IndexMap<NodeFeature, usize>,
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
        let mut features_to_nodes_map = IndexMap::new();

        for feature in NodeFeature::variants() {
            features_to_nodes_map.insert(feature, Vec::new());
        }

        // Convert a Vec<IndexMap<NodeFeature, Value>> into a Vec<IndexMap<NodeFeature,
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
            let value_counts = counters
                .into_iter()
                .sorted_unstable_by(|(feat1, cnt1), (feat2, cnt2)| {
                    let cmp1 = cnt2.partial_cmp(cnt1).unwrap();
                    if cmp1 == Ordering::Equal {
                        feat1.partial_cmp(feat2).unwrap_or(Ordering::Equal)
                    } else {
                        cmp1
                    }
                })
                .collect::<Vec<_>>();

            (value.0.clone(), Self::nakamoto(&only_counter), value_counts)
        });

        let scores = nakamoto_calc
            .clone()
            .map(|(f, n, _)| (f, n.0 as f64))
            .collect::<IndexMap<NodeFeature, f64>>();

        let controlled_nodes = nakamoto_calc.clone().map(|(f, n, _)| (f, n.1)).collect::<IndexMap<NodeFeature, usize>>();

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
    pub fn new_from_nodes<'a>(nodes: impl IntoIterator<Item = &'a Node> + Clone) -> Self {
        let mut memoize_key = AHasher::default();
        let nodes_iter = nodes.clone().into_iter();

        for node in nodes_iter.sorted_by_cached_key(|n| n.principal) {
            for byte in node.principal.0.as_slice() {
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
                                    *memoize_req.borrow_mut() = 0;
                                    *memoize_hit.borrow_mut() = 0;
                                }
                                score.clone()
                            }
                            None => {
                                let score = Self::new_from_slice_node_features(&nodes.into_iter().map(|n| n.get_features()).collect::<Vec<_>>());
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

    /// Get a Map with all the features and the corresponding Nakamoto score
    pub fn scores_individual(&self) -> IndexMap<NodeFeature, f64> {
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
    ///
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

    pub fn describe_difference_from(&self, other: &NakamotoScore) -> (Option<Ordering>, String) {
        // Prefer higher score across all features
        let mut cmp = self.score_avg_log2().partial_cmp(&other.score_avg_log2());

        if cmp != Some(Ordering::Equal) {
            return (
                cmp,
                if cmp == Some(Ordering::Less) {
                    format!(
                        "(gets worse) the average log2 of Nakamoto Coefficients across all features decreases from {:.4} to {:.4}",
                        other.score_avg_log2().unwrap_or(0.0),
                        self.score_avg_log2().unwrap_or(0.0)
                    )
                } else {
                    format!(
                        "(gets better) the average log2 of Nakamoto Coefficients across all features increases from {:.4} to {:.4}",
                        other.score_avg_log2().unwrap_or(0.0),
                        self.score_avg_log2().unwrap_or(0.0)
                    )
                },
            );
        }

        // Try to pick the candidate that *reduces* the number of nodes
        // controlled by the top actors
        cmp = other.critical_features_num_nodes().partial_cmp(&self.critical_features_num_nodes());

        if cmp != Some(Ordering::Equal) {
            let val_self = self.critical_features_num_nodes();
            let val_other = other.critical_features_num_nodes();
            return (
                cmp,
                if val_self[0] != val_other[0] {
                    if val_other[0] > val_self[0] {
                        format!(
                            "(gets better) the number of nodes controlled by dominant NPs decreases from {} to {}",
                            val_other[0], val_self[0]
                        )
                    } else {
                        format!(
                            "(gets worse) the number of nodes controlled by dominant NPs increases from {} to {}",
                            val_other[0], val_self[0]
                        )
                    }
                } else if val_other[1] > val_self[1] {
                    format!(
                        "(gets better) the number of nodes controlled by dominant Country actors decreases from {} to {}",
                        val_other[1], val_self[1]
                    )
                } else {
                    format!(
                        "(gets worse) the number of nodes controlled by dominant Country actors increases from {} to {}",
                        val_other[1], val_self[1]
                    )
                },
            );
        }

        // Compare the number of unique actors for the critical features
        // E.g. self has 5 NPs and other has 4 NPs ==> prefer self
        cmp = self
            .critical_features_unique_actors()
            .partial_cmp(&other.critical_features_unique_actors());

        if cmp != Some(Ordering::Equal) {
            let val_self = self.critical_features_unique_actors();
            let val_other = other.critical_features_unique_actors();
            return (
                cmp,
                if val_self[0] != val_other[0] {
                    if val_other[0] < val_self[0] {
                        format!(
                            "(gets better) the number of different NP actors increases from {} to {}",
                            val_other[0], val_self[0]
                        )
                    } else {
                        format!(
                            "(gets worse) the number of different NP actors decreases from {} to {}",
                            val_other[0], val_self[0]
                        )
                    }
                } else if val_other[1] < val_self[1] {
                    format!(
                        "(gets better) the number of different Country actors increases from {} to {}",
                        val_other[1], val_self[1]
                    )
                } else {
                    format!(
                        "(gets worse) the number of different Country actors decreases from {} to {}",
                        val_other[1], val_self[1]
                    )
                },
            );
        }

        // Compare the count of below-average coefficients
        // and prefer candidates that decrease the number of low-value coefficients
        let c1 = self.coefficients.values().filter(|c| **c < 3.0).count();
        let c2 = other.coefficients.values().filter(|c| **c < 3.0).count();
        cmp = c2.partial_cmp(&c1);

        if cmp != Some(Ordering::Equal) {
            return (
                cmp,
                if cmp == Some(Ordering::Less) {
                    format!(
                        "(gets better) the number of Nakamoto coefficients with extremely low values decreases from {} to {}",
                        c2, c1
                    )
                } else {
                    format!(
                        "(gets worse) the number of Nakamoto coefficients with extremely low values increases from {} to {}",
                        c2, c1
                    )
                },
            );
        }

        // If the worst feature is the same for both candidates
        // => prefer candidates that maximizes all features
        for feature in NodeFeature::variants() {
            if feature == NodeFeature::Continent {
                // Skip the continent feature as it is not used in the Nakamoto score
                continue;
            }
            let c1 = self.coefficients.get(&feature).unwrap_or(&1.0);
            let c2 = other.coefficients.get(&feature).unwrap_or(&1.0);
            if *c1 < 3.0 || *c2 < 3.0 {
                // Ensure that the new candidate does not decrease the critical features (that
                // are below the average)
                cmp = c2.partial_cmp(c1);

                if cmp != Some(Ordering::Equal) {
                    return (
                        cmp,
                        format!(
                            "the Nakamoto coefficient value for feature {} changes from {:.2} to {:.2}",
                            feature, c2, c1
                        ),
                    );
                }
            }
        }

        // And finally try to increase the linear average
        match self.score_avg_linear().partial_cmp(&other.score_avg_linear()) {
            Some(Ordering::Equal) => (Some(Ordering::Equal), "equal decentralization across all features".to_string()),
            Some(cmp) => (
                Some(cmp),
                format!(
                    "the linear average of Nakamoto coefficients across all features changes from {:.4} to {:.4}",
                    other.score_avg_linear(),
                    self.score_avg_linear()
                ),
            ),
            None => (None, String::new()),
        }
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
        self.describe_difference_from(other).0
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
            Some(v) => format!("{:0.4}", v),
            None => "undefined".to_string(),
        };
        write!(
            f,
            "NakamotoScore: min {:0.2} avg log2 {} #crit nodes {:?} # crit uniq {:?} #crit coeff {} avg linear {:0.4}",
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

    use crate::network::{DecentralizedSubnet, NetworkHealRequest, NetworkHealSubnet, SubnetChangeRequest};
    use ic_base_types::PrincipalId;
    use ic_management_types::HealthStatus;
    use indexmap::IndexMap;
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
            coefficients: IndexMap::from([
                (NodeFeature::NodeProvider, 1.),
                (NodeFeature::NodeOperator, 1.),
                (NodeFeature::DataCenter, 1.),
                (NodeFeature::DataCenterOwner, 1.),
                (NodeFeature::Area, 1.),
                (NodeFeature::Country, 1.),
            ]),
            value_counts: IndexMap::new(),
            controlled_nodes: IndexMap::from([
                (NodeFeature::NodeProvider, 1),
                (NodeFeature::NodeOperator, 1),
                (NodeFeature::DataCenter, 1),
                (NodeFeature::DataCenterOwner, 1),
                (NodeFeature::Area, 1),
                (NodeFeature::Country, 1),
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
            let node = Node::new_test_node(i as u64, node_features, dfinity_owned);
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
            let node = Node::new_test_node((node_number_start + i) as u64, node_features, dfinity_owned);
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
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
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
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
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
            (
                1000,
                vec!["Subnet should have 1 DFINITY-owned node(s) for subnet recovery, got 0".to_string()]
            )
        );
    }

    #[test]
    fn extend_feature_set_group() {
        let subnet_initial = new_test_subnet(0, 12, 1);
        let nodes_initial = subnet_initial.nodes.clone();
        let nodes_available = new_test_nodes("spare", 1, 0);
        let all_nodes = nodes_initial.iter().chain(nodes_available.iter()).cloned().collect::<Vec<_>>();

        let extended_subnet = subnet_initial.subnet_with_more_nodes(1, &nodes_available, &all_nodes).unwrap();
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
                1070,
                vec![
                    "Country US controls 9 of nodes, which is higher than target of 2 for the subnet. Applying penalty of 70.".to_string(),
                    "NodeFeature country controls 9 of nodes, which is > 8 (2/3 of all) nodes".to_string()
                ]
            )
        );
        let nodes_available = new_test_nodes_with_overrides("spare", 13, 3, 0, (&NodeFeature::Country, &["US", "RO", "JP"]));
        let all_nodes = nodes_available.iter().chain(subnet_initial.nodes.iter()).cloned().collect::<Vec<_>>();

        let health_of_nodes = all_nodes.iter().map(|n| (n.principal, HealthStatus::Healthy)).collect::<IndexMap<_, _>>();

        println!(
            "initial {} Countries {:?}",
            subnet_initial,
            subnet_initial
                .nodes
                .iter()
                .map(|n| n.get_feature(&NodeFeature::Country))
                .collect::<Vec<_>>()
        );

        let subnet_change_req = SubnetChangeRequest::new(subnet_initial, nodes_available, Vec::new(), Vec::new(), Vec::new());
        let subnet_change = subnet_change_req.optimize(2, &[], &health_of_nodes, vec![], &all_nodes).unwrap();
        for log in subnet_change.after().run_log.iter() {
            println!("{}", log);
        }
        let optimized_subnet = subnet_change.after();

        let countries_after = optimized_subnet
            .nodes
            .iter()
            .map(|n| n.get_feature(&NodeFeature::Country).unwrap())
            .sorted()
            .collect::<Vec<_>>();

        println!("optimized {} Countries {:?}", optimized_subnet, countries_after);

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
            (
                10020,
                vec![
                    "node_provider NP2 controls 3 of nodes, which is higher than target of 1 for the subnet. Applying penalty of 20.".to_string(),
                    "A single Node Provider can halt the subnet".to_string()
                ]
            )
        );
        let nodes_available = new_test_nodes_with_overrides("spare", 7, 2, 0, (&NodeFeature::NodeProvider, &["NP6", "NP7"]));
        let all_nodes = nodes_available.iter().chain(subnet_initial.nodes.iter()).cloned().collect::<Vec<_>>();
        let health_of_nodes = all_nodes.iter().map(|n| (n.principal, HealthStatus::Healthy)).collect::<IndexMap<_, _>>();

        println!(
            "initial {} NPs {:?}",
            subnet_initial,
            subnet_initial
                .nodes
                .iter()
                .map(|n| n.get_feature(&NodeFeature::NodeProvider))
                .collect::<Vec<_>>()
        );

        let subnet_change_req = SubnetChangeRequest::new(subnet_initial, nodes_available, Vec::new(), Vec::new(), Vec::new());
        let subnet_change = subnet_change_req.optimize(2, &[], &health_of_nodes, vec![], &all_nodes).unwrap();
        println!("Replacement run log:");
        for line in subnet_change.after().run_log.iter() {
            println!("{}", line);
        }
        let optimized_subnet = subnet_change.after();

        let nps_after = optimized_subnet
            .nodes
            .iter()
            .map(|n| n.get_feature(&NodeFeature::NodeProvider).unwrap())
            .sorted()
            .collect::<Vec<_>>();

        println!("optimized {} NPs {:?}", optimized_subnet, nps_after);

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
        assert_eq!(
            subnet_initial.check_business_rules().unwrap(),
            (
                20,
                vec!["node_provider NP2 controls 2 of nodes, which is higher than target of 1 for the subnet. Applying penalty of 10.".to_string(), "node_provider NP4 controls 2 of nodes, which is higher than target of 1 for the subnet. Applying penalty of 10.".to_string()]
            )
        );

        // There are 2 spare nodes, but both are DFINITY
        let nodes_available = new_test_nodes_with_overrides("spare", 7, 2, 2, (&NodeFeature::NodeProvider, &["NP6", "NP7"]));
        let all_nodes = nodes_available.iter().chain(subnet_initial.nodes.iter()).cloned().collect::<Vec<_>>();
        let health_of_nodes = all_nodes.iter().map(|n| (n.principal, HealthStatus::Healthy)).collect::<IndexMap<_, _>>();

        println!(
            "initial {} NPs {:?}",
            subnet_initial,
            subnet_initial
                .nodes
                .iter()
                .map(|n| n.get_feature(&NodeFeature::NodeProvider))
                .collect::<Vec<_>>()
        );

        let subnet_change_req = SubnetChangeRequest::new(subnet_initial, nodes_available, Vec::new(), Vec::new(), Vec::new());
        let subnet_change = subnet_change_req.optimize(2, &[], &health_of_nodes, vec![], &all_nodes).unwrap();

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

        // There is still only one DFINITY-owned node in the subnet
        assert_eq!(
            1,
            optimized_subnet
                .nodes
                .iter()
                .map(|n| n.dfinity_owned.unwrap_or_default() as u32)
                .sum::<u32>()
        );
    }

    #[test]
    fn subnet_uzr34_extend() {
        // Read the subnet snapshot from a file
        let subnet_all =
            serde_json::from_str::<ic_management_types::Subnet>(include_str!("../../test_data/subnet-uzr34.json")).expect("failed to read test data");
        // Convert the subnet snapshot to the "Subnet" struct
        let subnet_all: DecentralizedSubnet = DecentralizedSubnet::from(subnet_all);
        let re_unhealthy_nodes = Regex::new(r"^(gp7wd|e4ysi|qhz4y|2fbvp)-.+$").unwrap();
        let subnet_healthy = DecentralizedSubnet {
            id: subnet_all.id,
            nodes: subnet_all
                .nodes
                .iter()
                .filter(|n| !re_unhealthy_nodes.is_match(&n.principal.to_string()))
                .cloned()
                .collect(),
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            comment: None,
            run_log: Vec::new(),
        };

        let available_nodes = serde_json::from_str::<Vec<ic_management_types::Node>>(include_str!("../../test_data/available-nodes.json"))
            .expect("failed to read test data");

        let available_nodes = available_nodes
            .iter()
            .sorted_by(|a, b| a.principal.cmp(&b.principal))
            .filter(|n| n.subnet_id.is_none() && n.proposal.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let all_nodes = available_nodes.iter().chain(subnet_all.nodes.iter()).cloned().collect::<Vec<_>>();

        subnet_healthy.check_business_rules().expect("Check business rules failed");

        println!("Initial subnet {}", subnet_healthy);
        println!("Check business rules: {:?}", subnet_healthy.check_business_rules());
        let nakamoto_score_before = subnet_healthy.nakamoto_score();
        println!("NakamotoScore before {}", nakamoto_score_before);

        let extended_subnet = subnet_healthy.subnet_with_more_nodes(4, &available_nodes, &all_nodes).unwrap();
        println!("{}", extended_subnet);
        let nakamoto_score_after = extended_subnet.nakamoto_score();
        println!("NakamotoScore after {}", nakamoto_score_after);

        // Check against the close-to-optimal values obtained by data analysis
        assert!(nakamoto_score_after.critical_features_num_nodes()[0] <= 25);
        assert!(nakamoto_score_after.score_avg_linear() >= 3.0);
        assert!(nakamoto_score_after.score_avg_log2() >= Some(1.32));
    }

    #[test]
    fn test_extend_empty_subnet() {
        let available_nodes = (0..20)
            .map(|i| Node::new_test_node(i, NodeFeatures::new_test_feature_set(&format!("foo{i}")), i % 10 == 0))
            .collect::<Vec<_>>();
        let empty_subnet = DecentralizedSubnet::default();

        let want_subnet_size = 13;
        let new_subnet_result = empty_subnet.subnet_with_more_nodes(want_subnet_size, &available_nodes, &available_nodes);
        assert!(new_subnet_result.is_ok(), "error: {:?}", new_subnet_result.err());

        let new_subnet = new_subnet_result.unwrap();
        assert_eq!(new_subnet.nodes.len(), want_subnet_size)
    }

    #[test]
    fn test_european_subnet_european_nodes_good() {
        let subnet_initial = new_test_subnet_with_overrides(0, 0, 7, 1, (&NodeFeature::Country, &["AT", "BE", "DE", "ES", "FR", "IT", "CH"]))
            .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        assert_eq!(subnet_initial.check_business_rules().unwrap(), (0, vec![]));
    }

    #[test]
    fn test_european_subnet_european_nodes_bad_1() {
        let subnet_mix = new_test_subnet_with_overrides(1, 0, 7, 1, (&NodeFeature::Country, &["AT", "BE", "DE", "ES", "FR", "IT", "IN"]))
            .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        assert_eq!(
            subnet_mix.check_business_rules().unwrap(),
            (1000, vec!["European subnet has 1 non-European node(s)".to_string()])
        );
    }
    #[test]
    fn test_european_subnet_european_nodes_bad_2() {
        let subnet_mix = new_test_subnet_with_overrides(1, 0, 7, 1, (&NodeFeature::Country, &["AT", "BE", "DE", "ES", "US", "IN", "AR"]))
            .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        assert_eq!(
            subnet_mix.check_business_rules().unwrap(),
            (3000, vec!["European subnet has 3 non-European node(s)".to_string()])
        );
    }

    #[test]
    fn test_network_heal_subnets_ord() {
        let not_important_small = new_test_subnet(0, 13, 0)
            .with_subnet_id(PrincipalId::from_str("k44fs-gm4pv-afozh-rs7zw-cg32n-u7xov-xqyx3-2pw5q-eucnu-cosd4-uqe").unwrap());
        let not_important_small = NetworkHealSubnet {
            name: String::from("App 20"),
            decentralized_subnet: not_important_small,
            unhealthy_nodes: vec![],
            cordoned_nodes: vec![],
        };

        let not_important_large = new_test_subnet(0, 28, 0)
            .with_subnet_id(PrincipalId::from_str("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe").unwrap());
        let not_important_large = NetworkHealSubnet {
            name: String::from("European"),
            decentralized_subnet: not_important_large,
            unhealthy_nodes: vec![],
            cordoned_nodes: vec![],
        };

        let important =
            serde_json::from_str::<ic_management_types::Subnet>(include_str!("../../test_data/subnet-uzr34.json")).expect("failed to read test data");
        let important = NetworkHealSubnet {
            name: important.metadata.name.clone(),
            decentralized_subnet: DecentralizedSubnet::from(important),
            unhealthy_nodes: vec![],
            cordoned_nodes: vec![],
        };

        let unordered = vec![not_important_small.clone(), important.clone(), not_important_large.clone()];
        let healing_order = unordered.clone().into_iter().sorted_by(|a, b| a.cmp(b).reverse()).collect_vec();

        assert_eq!(vec![important, not_important_large, not_important_small], healing_order);
    }

    #[tokio::test]
    async fn test_network_heal() {
        let nodes_available = new_test_nodes("spare", 10, 2);

        let subnet =
            serde_json::from_str::<ic_management_types::Subnet>(include_str!("../../test_data/subnet-uzr34.json")).expect("failed to read test data");
        let unhealthy_principals = [
            "e4ysi-xp4fs-5ckcv-7e76q-edydw-ak6le-2acyt-k7udb-lj2vo-fqhhx-vqe",
            "aefqq-d7ldg-ljk5s-cmnxk-qqu7c-tw52l-74g3m-xxl5d-ag4ia-dxubz-wae",
        ]
        .into_iter()
        .flat_map(PrincipalId::from_str)
        .collect_vec();

        let all_nodes = nodes_available.iter().chain(subnet.nodes.iter()).cloned().collect::<Vec<_>>();
        let health_of_nodes = all_nodes
            .iter()
            .map(|n| {
                let node_id = n.principal;
                if unhealthy_principals.contains(&node_id) {
                    (node_id, HealthStatus::Dead)
                } else {
                    (node_id, HealthStatus::Healthy)
                }
            })
            .collect::<IndexMap<_, _>>();
        let mut important = IndexMap::new();

        important.insert(subnet.principal, subnet);

        let network_heal_response = NetworkHealRequest::new(important.clone())
            .heal_and_optimize(nodes_available.clone(), &health_of_nodes, vec![], &all_nodes, false, false)
            .await
            .unwrap();
        let result = network_heal_response.first().unwrap().clone();

        for unhealthy in unhealthy_principals.to_vec().iter() {
            assert!(result.node_ids_removed.contains(unhealthy));
        }
    }

    #[test]
    fn test_subnet_rescue() {
        let nodes_available = new_test_nodes("spare", 10, 1);
        let subnet_initial = new_test_subnet_with_overrides(0, 11, 7, 1, (&NodeFeature::Country, &["CH", "CA", "CA", "CA", "CA", "CA", "BE"]));
        let all_nodes = nodes_available.iter().chain(subnet_initial.nodes.iter()).cloned().collect::<Vec<_>>();
        let health_of_nodes = all_nodes.iter().map(|n| (n.principal, HealthStatus::Healthy)).collect::<IndexMap<_, _>>();

        let change_initial = SubnetChangeRequest::new(subnet_initial.clone(), nodes_available, Vec::new(), Vec::new(), Vec::new());

        let with_keeping_features = change_initial
            .clone()
            .keeping_from_used(vec!["CH".to_string()])
            .rescue(&health_of_nodes, vec![], &all_nodes)
            .unwrap();

        assert_eq!(with_keeping_features.added().len(), 4);
        assert_eq!(
            with_keeping_features
                .new_nodes
                .iter()
                .filter(|n| n.get_feature(&NodeFeature::Country).unwrap() == *"CH")
                .collect_vec()
                .len(),
            1
        );

        let node_to_keep = subnet_initial.nodes.first().unwrap();
        let with_keeping_principals = change_initial
            .clone()
            .keeping_from_used(vec!["CH".to_string()])
            .rescue(&health_of_nodes, vec![], &all_nodes)
            .unwrap();

        assert_eq!(with_keeping_principals.added().len(), 4);
        assert_eq!(
            with_keeping_principals
                .new_nodes
                .iter()
                .filter(|n| n.principal == node_to_keep.principal)
                .collect_vec()
                .len(),
            1
        );

        let rescue_all = change_initial.clone().rescue(&health_of_nodes, vec![], &all_nodes).unwrap();

        assert_eq!(rescue_all.added().len(), 5);
        assert_eq!(rescue_all.removed().len(), 5);
    }

    #[test]
    fn test_resize() {
        let subnet_initial = new_test_subnet(0, 24, 0);
        let all_nodes = subnet_initial.nodes.clone();
        let health_of_nodes = all_nodes.iter().map(|n| (n.principal, HealthStatus::Healthy)).collect::<IndexMap<_, _>>();
        let change_initial = SubnetChangeRequest::new(subnet_initial.clone(), Vec::new(), Vec::new(), Vec::new(), Vec::new());

        let after_resize = change_initial.resize(2, 2, 0, &health_of_nodes, vec![], &all_nodes).unwrap();

        assert_eq!(subnet_initial.nodes.len(), after_resize.new_nodes.len());

        assert_eq!(after_resize.added_nodes.len(), 0);
        assert_eq!(after_resize.removed_nodes.len(), 0);
    }
}
