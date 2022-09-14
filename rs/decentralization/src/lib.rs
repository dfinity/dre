pub mod nakamoto;
pub mod network;
use colored::Colorize;
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};

use ic_base_types::PrincipalId;
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

impl From<&network::SubnetChange> for SubnetChangeResponse {
    fn from(change: &network::SubnetChange) -> Self {
        Self {
            added: change.added().iter().map(|n| n.id).collect(),
            removed: change.removed().iter().map(|n| n.id).collect(),
            subnet_id: if change.id == Default::default() {
                None
            } else {
                Some(change.id)
            },
            score_before: nakamoto::NakamotoScore::new_from_nodes(&change.old_nodes),
            score_after: nakamoto::NakamotoScore::new_from_nodes(&change.new_nodes),
            motivation: None,
            feature_diff: change.new_nodes.iter().fold(
                change.old_nodes.iter().fold(
                    nakamoto::Feature::variants()
                        .into_iter()
                        .map(|f| (f, FeatureDiff::new()))
                        .collect::<HashMap<nakamoto::Feature, FeatureDiff>>(),
                    |mut acc, n| {
                        for f in nakamoto::Feature::variants() {
                            acc.get_mut(&f).unwrap().entry(n.get_feature(&f)).or_insert((0, 0)).0 += 1;
                        }
                        acc
                    },
                ),
                |mut acc, n| {
                    for f in nakamoto::Feature::variants() {
                        acc.get_mut(&f).unwrap().entry(n.get_feature(&f)).or_insert((0, 0)).1 += 1;
                    }
                    acc
                },
            ),
        }
    }
}

impl Display for SubnetChangeResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Decentralization score changes:\n")?;
        let before_individual = self.score_before.scores_individual();
        let after_individual = self.score_after.scores_individual();
        self.score_before
            .scores_individual()
            .keys()
            .sorted()
            .map(|k| {
                let before = before_individual.get(k).unwrap();
                let after = after_individual.get(k).unwrap();
                let output = format!(
                    "{}: {:.2} -> {:.2}  {:>7}",
                    k,
                    before,
                    after,
                    format_args!("({:+.0}%)", ((after - before) / before) * 100.)
                );
                if before > after {
                    output.bright_red()
                } else if after > before {
                    output.bright_green()
                } else {
                    output.dimmed()
                }
            })
            .for_each(|s| writeln!(f, "{: >40}", s).expect("write failed"));

        let total_before = self.score_before.score_avg_linear();
        let total_after = self.score_after.score_avg_linear();
        let output = format!(
            "\tTotal: {:.2} -> {:.2}  ({:+.0}%)",
            total_before,
            total_after,
            ((total_after - total_before) / total_before) * 100.
        )
        .bold();

        writeln!(
            f,
            "\n{}\n",
            if total_before > total_after {
                output.red()
            } else if total_after > total_before {
                output.green()
            } else {
                output.dimmed()
            }
        )?;

        let feature_diff = BTreeMap::from_iter(self.feature_diff.iter());
        let rows = feature_diff.values().map(|diff| diff.len()).max().unwrap_or(0);
        let mut table = tabular::Table::new(
            &feature_diff
                .keys()
                .map(|_| "    {:<}  {:>}")
                .collect::<Vec<_>>()
                .join(""),
        );
        table.add_row(
            feature_diff
                .keys()
                .fold(tabular::Row::new(), |acc, k| acc.with_cell(k.to_string()).with_cell("")),
        );
        table.add_row(feature_diff.keys().fold(tabular::Row::new(), |acc, k| {
            acc.with_cell("-".repeat(k.to_string().len())).with_cell("")
        }));
        for i in 0..rows {
            table.add_row(feature_diff.values().fold(tabular::Row::new(), |acc, v| {
                let (value, change) = v
                    .iter()
                    .sorted()
                    .nth(i)
                    .map(|(k, (before, after))| {
                        (
                            k.to_string(),
                            match before.cmp(after) {
                                std::cmp::Ordering::Equal => format!("{}", before),
                                std::cmp::Ordering::Greater => format!("{} -> {}", before, after),
                                std::cmp::Ordering::Less => format!("{} -> {}", before, after),
                            },
                        )
                    })
                    .unwrap_or_default();
                acc.with_cell(value).with_cell(change)
            }));
        }

        writeln!(f, "{}", table)?;
        self.added.iter().zip(self.removed.iter()).for_each(|(a, r)| {
            writeln!(f, "{}{}", format!("  - {}", r).red(), format!("    + {}", a).green()).expect("write failed");
        });
        writeln!(f)
    }
}
