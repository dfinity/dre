use crate::network::Node;
use core::hash::Hash;
use counter::Counter;
use num_traits::{One, Zero};
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
    fn get_feature(&self, feature: Feature) -> Self::T;
    fn check_business_rules<IterType>(candidate: IterType) -> bool
    where
        IterType: IntoIterator<Item = Self>;
}

impl Decentralize for FeatureSet {
    type T = String;
    fn get_feature(&self, feature: Feature) -> String {
        self.get(&feature).unwrap_or(&"unknown".to_string()).to_string()
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

    fn get_feature(&self, feature: Feature) -> String {
        self.features.get_feature(feature)
    }

    fn check_business_rules<IterType>(candidate: IterType) -> bool
    where
        IterType: IntoIterator<Item = Self>,
    {
        let candidate_vec = candidate.into_iter().collect::<Vec<Self>>();
        candidate_vec.iter().filter(|x| x.dfinity_owned).count() >= 1
            && FeatureSet::check_business_rules(candidate_vec.into_iter().map(|x| x.features))
    }
}

pub type FeatureSet = HashMap<Feature, String>;

pub trait Extendable
where
    Self: IntoIterator,
    Self::Item: Decentralize,
{
    fn best_extension(self, size: usize, available: Vec<Self::Item>) -> Option<Vec<Self::Item>>;
    fn merge<Available>(self, other: Available) -> Self
    where
        Available: IntoIterator<Item = Self::Item>;
}

impl<T> Extendable for T
where
    T: IntoIterator + FromIterator<T::Item>,
    T::Item: Decentralize + Clone + PartialEq,
{
    fn best_extension(self, size: usize, mut available: Vec<T::Item>) -> Option<Vec<T::Item>> {
        if size == 0 {
            return Some(Vec::new());
        }
        let current = self.into_iter().collect::<Vec<_>>();
        let best = available
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let mut current = current.clone();
                current.push(node.clone());
                if T::Item::check_business_rules(current.clone()) {
                    Some((index, node.clone(), Score::from(current)))
                } else {
                    None
                }
            })
            .max_by_key(|(_, _, score)| (score.total * 100.) as u64);

        best.and_then(|(index, node, _)| {
            available.swap_remove(index);
            let mut current = current.clone();
            current.push(node.clone());
            current.best_extension(size - 1, available).map(|mut extension| {
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
pub struct Score {
    scores: HashMap<String, f64>,
    total: f64,
}

impl Score {
    /// Get a reference to the score's total.
    pub fn total(&self) -> f64 {
        self.total
    }

    /// Get individual scores.
    pub fn individual(&self) -> HashMap<String, f64> {
        self.scores.clone()
    }
}

pub trait Nakamoto
where
    Self: IntoIterator,
    Self::Item: std::cmp::Ord + num_traits::Num + Clone,
{
    fn nakamoto(self) -> (i32, Self::Item);
}

impl<T> Nakamoto for T
where
    T: IntoIterator,
    // Trait bound for "any numeric type that has a full ordering"
    // https://docs.rs/num-traits/latest/num_traits/trait.Num.html
    T::Item: std::cmp::Ord + num_traits::Num + Clone,
{
    fn nakamoto(self) -> (i32, T::Item) {
        // We collect here because Iterators are not orderable, and Vectors are.
        let mut values: Vec<T::Item> = self.into_iter().collect::<Vec<T::Item>>();
        // Sum of all values in our vector, which is the "number of objects" in our
        // nakamoto calculation. T::Item::zero() is just "whatever zero is for
        // this type of number"
        let len: T::Item = values.iter().fold(T::Item::zero(), |acc, x| acc + x.clone());
        // T::Item::one() = "whatever one is for this type of number".
        let three = T::Item::one() + T::Item::one() + T::Item::one();
        // Haltable is the number of objects that, if colluding - would break Byzantine
        // Fault Tolerance.
        let haltable = (len / three) + T::Item::one();
        // Reverse sort, go from biggest actor to smallest actor and the ultimate
        // nakamoto coefficient is however many actors I have to consider before
        // I break BFT
        values.sort_by(|a, b| b.cmp(a));
        let mut count = 0;
        let mut curr = T::Item::zero();
        for value in values {
            count += 1;
            curr = curr + value;
            if curr >= haltable {
                break;
            }
        }
        (count, haltable)
    }
}

impl<T> From<T> for Score
where
    T: IntoIterator,
    T::Item: Decentralize,
{
    fn from(feature_sets: T) -> Self {
        let mut deconstructed = HashMap::new();

        for feature in Feature::variants() {
            deconstructed.insert(feature.to_string(), Vec::new());
        }

        // Convert a Vec<HashMap<Feature, Value>> into a Vec<HashMap<Feature,
        // Vec<Values>>
        for features in feature_sets {
            for feature in Feature::variants() {
                let curr = deconstructed.get_mut(&feature.to_string()).unwrap();
                curr.push(features.get_feature(feature));
            }
        }

        let scores = deconstructed
            .iter()
            .map(|value| {
                // Turns a Vec<Features> into a Vec<(Feature, Number)> where "Number" is the
                // number of objects with that feature.
                let counter = value.1.iter().collect::<Counter<_>>();
                // Nakamoto as above.
                let (potential_nakamoto, haltable) = counter.values().copied().nakamoto();
                // We divide curr_nakamoto by haltable, which is "max nakamoto"
                (value.0.clone(), potential_nakamoto as f64 / haltable as f64)
            })
            .collect::<HashMap<String, f64>>();
        // Average the totals.
        let total: f64 = scores.values().copied().sum::<f64>() / scores.len() as f64;
        Score { scores, total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Decentralize, FeatureSet, Node};
    use include_dir::{include_dir, Dir};

    #[test]
    fn computes_nakamotos() {
        let test = vec![1, 1, 2, 3, 5, 1, 2];
        let out = test.nakamoto();
        assert_eq!((2, 6), out);
    }

    #[test]
    fn test_business_rules() {
        static TEST_CASES_DIR: Dir<'_> = include_dir!("decentralization/test_cases/nakamoto/business_rules");
        for case in TEST_CASES_DIR.dirs() {
            let feature_input: Vec<FeatureSet> =
                serde_json::from_slice(case.get_file(case.path().join("features.json")).unwrap().contents()).unwrap();
            let expected: bool =
                serde_json::from_slice(case.get_file(case.path().join("expected.json")).unwrap().contents()).unwrap();
            assert_eq!(expected, FeatureSet::check_business_rules(feature_input));
            let node_input: Vec<Node> =
                serde_json::from_slice(case.get_file(case.path().join("nodes.json")).unwrap().contents()).unwrap();
            assert_eq!(expected, Node::check_business_rules(node_input));
        }
    }
    #[test]
    fn score_from_features() {
        static TEST_CASES_DIR: Dir<'_> = include_dir!("decentralization/test_cases/nakamoto/score");
        for case in TEST_CASES_DIR.dirs() {
            let input: Vec<FeatureSet> = serde_json::from_slice(
                case.get_file(case.path().join("feature_set_group.json"))
                    .unwrap()
                    .contents(),
            )
            .unwrap();
            let score = Score::from(input);
            let expected: Score =
                serde_json::from_slice(case.get_file(case.path().join("expected.json")).unwrap().contents()).unwrap();
            assert_eq!(expected, score, "case: {}", case.path().display());
        }
    }

    #[test]
    fn extend_feature_set_group() {
        static TEST_CASES_DIR: Dir<'_> = include_dir!("decentralization/test_cases/nakamoto/extend");
        for case in TEST_CASES_DIR.dirs() {
            let input: Vec<FeatureSet> = serde_json::from_slice(
                case.get_file(case.path().join("feature_set_group_in.json"))
                    .unwrap()
                    .contents(),
            )
            .unwrap();

            let available: Vec<FeatureSet> = serde_json::from_slice(
                case.get_file(case.path().join("feature_set_group_available.json"))
                    .unwrap()
                    .contents(),
            )
            .unwrap();

            let add: usize =
                serde_json::from_slice(case.get_file(case.path().join("size.json")).unwrap().contents()).unwrap();

            let extension = input.best_extension(add, available).unwrap();

            let expected: Vec<FeatureSet> = serde_json::from_slice(
                case.get_file(case.path().join("feature_set_group_expected.json"))
                    .unwrap()
                    .contents(),
            )
            .unwrap();
            assert_eq!(expected, extension, "case: {}", case.path().display());
        }
    }
}
