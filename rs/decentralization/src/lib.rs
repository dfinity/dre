pub mod nakamoto;
pub mod network;
pub mod subnets;
use indexmap::IndexMap;
use itertools::Itertools;
use network::{DecentralizedSubnet, Node, SubnetChange};
use std::fmt::{Display, Formatter};

use ic_base_types::PrincipalId;
use ic_management_types::{HealthStatus, NodeFeature};
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct SubnetChangeResponse {
    pub nodes_old: Vec<Node>,
    pub node_ids_added: Vec<PrincipalId>,
    pub node_ids_removed: Vec<PrincipalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<PrincipalId>,
    pub health_of_nodes: IndexMap<PrincipalId, HealthStatus>,
    pub decentralization_impact: IndexMap<PrincipalId, String>,
    pub score_before: nakamoto::NakamotoScore,
    pub score_after: nakamoto::NakamotoScore,
    pub penalties_before_change: usize,
    pub penalties_after_change: usize,
    pub motivation: Option<String>,
    pub comment: Option<String>,
    pub run_log: Option<Vec<String>>,
    pub feature_diff: IndexMap<NodeFeature, FeatureDiff>,
    pub proposal_id: Option<u64>,
}

pub type FeatureDiff = IndexMap<String, (usize, usize)>;

impl SubnetChangeResponse {
    pub fn new(change: &SubnetChange, node_health: &IndexMap<PrincipalId, HealthStatus>, motivation: Option<String>) -> Self {
        let mut decentralization_impact = IndexMap::new();
        // Calculate decentralization impact for each removed node
        let mut nodes = change.old_nodes.iter().map(|n| (n.id, n.clone())).collect::<IndexMap<_, _>>();
        for node in change.removed().iter() {
            let subnet_before = DecentralizedSubnet::new_with_subnet_id_and_nodes(change.subnet_id, nodes.values().cloned().collect());
            nodes.shift_remove(&node.id);
            let subnet_after = DecentralizedSubnet::new_with_subnet_id_and_nodes(change.subnet_id, nodes.values().cloned().collect());
            let impact = subnet_after.nakamoto_score().describe_difference_from(&subnet_before.nakamoto_score()).1;
            decentralization_impact.insert(node.id, impact);
        }
        for node in change.added().iter() {
            let subnet_before = DecentralizedSubnet::new_with_subnet_id_and_nodes(change.subnet_id, nodes.values().cloned().collect());
            nodes.insert(node.id, node.clone());
            let subnet_after = DecentralizedSubnet::new_with_subnet_id_and_nodes(change.subnet_id, nodes.values().cloned().collect());
            let impact = subnet_after.nakamoto_score().describe_difference_from(&subnet_before.nakamoto_score()).1;
            decentralization_impact.insert(node.id, impact);
        }

        Self {
            nodes_old: change.old_nodes.clone(),
            node_ids_added: change.added().iter().map(|n| n.id).collect(),
            node_ids_removed: change.removed().iter().map(|n| n.id).collect(),
            subnet_id: if change.subnet_id == Default::default() {
                None
            } else {
                Some(change.subnet_id)
            },
            health_of_nodes: node_health.clone(),
            decentralization_impact,
            score_before: nakamoto::NakamotoScore::new_from_nodes(&change.old_nodes),
            score_after: nakamoto::NakamotoScore::new_from_nodes(&change.new_nodes),
            penalties_before_change: change.penalties_before_change,
            penalties_after_change: change.penalties_after_change,
            motivation,
            comment: change.comment.clone(),
            run_log: Some(change.run_log.clone()),
            feature_diff: change.new_nodes.iter().fold(
                change.old_nodes.iter().fold(
                    NodeFeature::variants()
                        .into_iter()
                        .map(|f| (f, FeatureDiff::new()))
                        .collect::<IndexMap<NodeFeature, FeatureDiff>>(),
                    |mut acc, n| {
                        for f in NodeFeature::variants() {
                            acc.get_mut(&f).unwrap().entry(n.get_feature(&f)).or_insert((0, 0)).0 += 1;
                        }
                        acc
                    },
                ),
                |mut acc, n| {
                    for f in NodeFeature::variants() {
                        acc.get_mut(&f).unwrap().entry(n.get_feature(&f)).or_insert((0, 0)).1 += 1;
                    }
                    acc
                },
            ),
            proposal_id: None,
        }
    }
    pub fn with_motivation(self, motivation: String) -> Self {
        SubnetChangeResponse {
            motivation: Some(motivation),
            ..self
        }
    }
}

impl Display for SubnetChangeResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Decentralization Nakamoto coefficient changes for subnet `{}`:\n```",
            self.subnet_id.unwrap_or_default()
        )?;
        let before_individual = self.score_before.scores_individual();
        let after_individual = self.score_after.scores_individual();
        self.score_before
            .scores_individual()
            .keys()
            .sorted()
            .map(|k| {
                let before = before_individual.get(k).unwrap();
                let after = after_individual.get(k).unwrap();
                format!(
                    "{}: {:.2} -> {:.2}  {:>7}",
                    k,
                    before,
                    after,
                    format_args!("({:+.0}%)", ((after - before) / before) * 100.).to_string()
                )
            })
            .for_each(|s| writeln!(f, "{: >40}", s).expect("write failed"));

        let total_before = self.score_before.score_avg_linear();
        let total_after = self.score_after.score_avg_linear();
        writeln!(
            f,
            "```\n\n**Mean Nakamoto comparison:** {:.2} -> {:.2}  ({:+.0}%)\n\nOverall replacement impact: {}",
            total_before,
            total_after,
            ((total_after - total_before) / total_before) * 100.,
            self.score_after.describe_difference_from(&self.score_before).1
        )?;

        if self.penalties_before_change != self.penalties_after_change || self.penalties_after_change > 0 {
            writeln!(
                f,
                "\nImpact on business rules penalties: {} -> {}",
                self.penalties_before_change, self.penalties_after_change
            )?;
        }

        writeln!(f, "\n\n# Details\n\nNodes removed:")?;
        for node_id in &self.node_ids_removed {
            let health = self
                .health_of_nodes
                .get(node_id)
                .map(|h| h.to_string().to_lowercase())
                .unwrap_or("unknown".to_string());
            let desc = self.decentralization_impact.get(node_id).cloned().unwrap_or("unknown".to_string());
            writeln!(f, "- `{}` [health: {}, impact on decentralization: {}]", node_id, health, desc).expect("write failed");
        }
        writeln!(f, "\nNodes added:")?;
        for node_id in &self.node_ids_added {
            let health = self
                .health_of_nodes
                .get(node_id)
                .map(|h| h.to_string().to_lowercase())
                .unwrap_or("unknown".to_string());
            let desc = self.decentralization_impact.get(node_id).cloned().unwrap_or("unknown".to_string());
            writeln!(f, "- `{}` [health: {}, impact on decentralization: {}]", node_id, health, desc).expect("write failed");
        }

        let rows = self.feature_diff.values().map(|diff| diff.len()).max().unwrap_or(0);
        let mut table = tabular::Table::new(&self.feature_diff.keys().map(|_| "    {:<}  {:>}").collect::<Vec<_>>().join(""));
        table.add_row(
            self.feature_diff
                .keys()
                .fold(tabular::Row::new(), |acc, k| acc.with_cell(k.to_string()).with_cell("")),
        );
        table.add_row(
            self.feature_diff
                .keys()
                .fold(tabular::Row::new(), |acc, k| acc.with_cell("-".repeat(k.to_string().len())).with_cell("")),
        );
        for i in 0..rows {
            table.add_row(self.feature_diff.values().fold(tabular::Row::new(), |acc, v| {
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

        writeln!(f, "\n\n```\n{}```\n", table)?;

        if let Some(comment) = &self.comment {
            writeln!(f, "### Business rules analysis\n{}", comment)?;
        }

        Ok(())
    }
}
