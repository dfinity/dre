use super::*;
use crate::rewards_calculator::builder::RewardsCalculatorBuilder;
use crate::rewards_calculator_results::{Percent, RewardsCalculatorResults};
use crate::types::{NodeMetricsDailyRaw, RewardPeriod, RewardableNode, UnixTsNanos, NANOS_PER_DAY};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use ic_protobuf::registry::node_rewards::v2::{NodeRewardRate, NodeRewardRates, NodeRewardsTable};
use itertools::Itertools;
use maplit::btreemap;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

pub fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

pub fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

impl Default for NodeMetricsDailyRaw {
    fn default() -> Self {
        Self {
            node_id: node_id(0),
            num_blocks_proposed: 0,
            num_blocks_failed: 0,
        }
    }
}

impl Default for RewardableNode {
    fn default() -> Self {
        Self {
            node_id: NodeId::from(PrincipalId::default()),
            rewardable_from: 0.into(),
            rewardable_to: 0.into(),
            region: Default::default(),
            node_type: Default::default(),
            dc_id: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct RewardCalculatorRunnerTest {
    reward_period: Option<RewardPeriod>,
    node_rewards_table: Option<NodeRewardsTable>,
    rewardable_nodes: Option<HashSet<RewardableNode>>,
    daily_data: HashMap<UnixTsNanos, Vec<(SubnetId, NodeId, u64, u64)>>,
}

impl RewardCalculatorRunnerTest {
    pub fn with_reward_period(mut self, start_ts: UnixTsNanos, end_ts: UnixTsNanos) -> Self {
        self.reward_period = Some(RewardPeriod::new(start_ts, end_ts).unwrap());
        self
    }
}

impl RewardCalculatorRunnerTest {
    pub fn with_rewards_rates(mut self, region: &str, node_types: Vec<&str>, rate: u64, coeff: u64) -> Self {
        let mut rates: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
        for node_type in node_types {
            rates.insert(
                node_type.to_string(),
                NodeRewardRate {
                    xdr_permyriad_per_node_per_month: rate,
                    reward_coefficient_percent: Some(coeff as i32),
                },
            );
        }
        let mut node_rewards_table = NodeRewardsTable::default();
        node_rewards_table.table.insert(region.to_string(), NodeRewardRates { rates });

        self.node_rewards_table = Some(node_rewards_table);
        self
    }
    pub fn with_data_next_day(mut self, data: Vec<(SubnetId, NodeId, u64, u64)>) -> Self {
        if self.daily_data.is_empty() {
            self.daily_data.insert(0, data);
        } else {
            let next_day = self.daily_data.keys().max().unwrap() + NANOS_PER_DAY;
            self.daily_data.insert(next_day, data);
        }

        self
    }

    pub fn with_all_days_data(self, data: Vec<Vec<(u64, u64, u64, u64)>>) -> Self {
        data.into_iter().fold(self, |builder, day_data| {
            let day_data_processed = day_data
                .into_iter()
                .map(|(subnet_id_u64, node_id_u64, proposed, failed)| (subnet_id(subnet_id_u64), node_id(node_id_u64), proposed, failed))
                .collect();
            builder.with_data_next_day(day_data_processed)
        })
    }

    pub fn with_node_metrics(mut self, node_id: NodeId, metrics: Vec<(UnixTsNanos, SubnetId, u64, u64)>) -> Self {
        for (utc_day, subnet_id, proposed, failed) in metrics {
            let entry = self.daily_data.entry(utc_day).or_default();
            entry.push((subnet_id, node_id, proposed, failed));
        }

        self
    }

    pub fn with_rewardable_nodes(mut self, nodes: Vec<NodeId>, region: &str, node_type: NodeRewardType) -> Self {
        let period = self.reward_period.clone().unwrap();
        let start_ts = period.0.from;
        let end_ts = period.0.to;

        let rewardables = nodes.into_iter().map(|node_id| RewardableNode {
            node_id,
            region: Region(region.to_string()),
            node_type,
            rewardable_from: start_ts,
            rewardable_to: end_ts,
            ..Default::default()
        });
        if let Some(rewardable_nodes) = self.rewardable_nodes.as_mut() {
            rewardable_nodes.extend(rewardables);
        } else {
            self.rewardable_nodes = Some(rewardables.collect());
        }
        self
    }

    pub fn build_and_run(self) -> RewardsCalculatorResults {
        let reward_period = self.reward_period.unwrap_or({
            let start_ts = self.daily_data.keys().min().unwrap();
            let end_ts = self.daily_data.keys().max().unwrap();
            RewardPeriod::new(*start_ts, *end_ts).unwrap()
        });

        let rewardables: Vec<_> = self
            .rewardable_nodes
            .unwrap_or(
                self.daily_data
                    .values()
                    .flat_map(|nodes| {
                        nodes.iter().map(|node| RewardableNode {
                            node_id: node.1,
                            rewardable_from: reward_period.0.from,
                            rewardable_to: reward_period.0.to,
                            node_type: NodeRewardType::Type1,
                            ..Default::default()
                        })
                    })
                    .collect::<HashSet<_>>(),
            )
            .into_iter()
            .collect();

        let rewardable_nodes_per_provider = btreemap! {
            PrincipalId::new_anonymous() => ProviderRewardableNodes {
                provider_id: PrincipalId::new_anonymous(),
                rewardable_nodes: rewardables,
            }
        };

        let subnets_metrics: HashMap<SubnetMetricsDailyKey, Vec<NodeMetricsDailyRaw>> = self
            .daily_data
            .into_iter()
            .flat_map(|(ts, metrics)| {
                metrics.into_iter().map(move |(subnet_id, node_id, proposed, failed)| {
                    (
                        SubnetMetricsDailyKey { subnet_id, day: ts.into() },
                        NodeMetricsDailyRaw {
                            node_id,
                            num_blocks_proposed: proposed,
                            num_blocks_failed: failed,
                        },
                    )
                })
            })
            .into_group_map();

        RewardsCalculatorBuilder {
            reward_period,
            rewardable_nodes_per_provider,
            daily_metrics_by_subnet: subnets_metrics.into_iter().collect(),
            rewards_table: self.node_rewards_table.unwrap_or_default(),
        }
        .build()
        .unwrap()
        .calculate_rewards_per_provider()
        .pop_first()
        .unwrap()
        .1
    }

    pub fn for_scenario_1() -> Self {
        // Each inner vector represents one day (days 0 through 3)
        // Tuple: (subnet_id, node_id, num_blocks_proposed, num_blocks_failed)
        // Using a total of 100 blocks for each node for easy interpretation
        let input = vec![
            // day 0
            vec![
                (1, 1, 70, 30), // FR = 0.3 = 30/100
                (1, 2, 60, 40), // FR = 0.4 = 40/100
                (1, 3, 50, 50), // FR = 0.5 = 50/100
                (2, 5, 66, 34), // FR = 0.34 = 34/100
                (2, 6, 80, 20), // FR = 0.2 = 20/100
                (2, 7, 80, 20), // FR = 0.2 = 20/100
            ],
            // day 1
            vec![
                (1, 1, 80, 20), // FR = 0.2 = 20/100
                (1, 2, 90, 10), // FR = 0.1 = 10/100
                (1, 3, 100, 0), // FR = 0.0 = 0/100
                (2, 4, 50, 50), // FR = 0.5 = 50/100
                (2, 5, 0, 100), // FR = 1.0 = 100/100
                (2, 6, 30, 70), // FR = 0.7 = 70/100
                (2, 7, 30, 70), // FR = 0.7 = 70/100
            ],
            // day 2
            vec![
                (1, 1, 90, 10), // FR = 0.1 = 10/100
                (1, 2, 80, 20), // FR = 0.2 = 20/100
                (1, 3, 70, 30), // FR = 0.3 = 30/100
                (2, 4, 60, 40), // FR = 0.4 = 40/100
                (2, 5, 0, 100), // FR = 1.0 = 100/100
                (2, 6, 80, 20), // FR = 0.2 = 20/100
                (2, 7, 90, 10), // FR = 0.1 = 10/100
            ],
            // day 3
            vec![
                (1, 2, 80, 20), // FR = 0.2 = 20/100
                (2, 3, 70, 30), // FR = 0.3 = 30/100
                (2, 4, 60, 40), // FR = 0.4 = 40/100
                (2, 6, 30, 70), // FR = 0.7 = 70/100
                (2, 7, 50, 50), // FR = 0.5 = 50/100
            ],
        ];

        RewardCalculatorRunnerTest::default().with_all_days_data(input)
    }
}

#[test]
fn test_calculates_node_failure_rates_correctly() {
    let results = RewardCalculatorRunnerTest::default()
        .with_node_metrics(node_id(0), vec![(0, subnet_id(2), 90, 10), (0, subnet_id(1), 1, 0)])
        .with_node_metrics(node_id(1), vec![(NANOS_PER_DAY, subnet_id(1), 60, 40)])
        .build_and_run();

    let nodes_results = results.results_by_node;

    let node_0_fr = &nodes_results.get(&node_id(0)).unwrap().daily_metrics.values().collect_vec();

    // Expected subnet 2 to be selected as the primary subnet because it has the highest number of proposed blocks
    assert_eq!(node_0_fr[0].subnet_assigned, subnet_id(2));
    assert_eq!(node_0_fr[0].original_fr.get(), dec!(0.1));
    assert_eq!(node_0_fr[0].relative_fr.get(), dec!(0));

    let node_1_fr = &nodes_results.get(&node_id(1)).unwrap().daily_metrics.values().collect_vec();

    assert_eq!(node_1_fr[0].subnet_assigned, subnet_id(1));
    assert_eq!(node_1_fr[0].original_fr.get(), dec!(0.4));
    assert_eq!(node_1_fr[0].relative_fr.get(), dec!(0));
}

#[test]
fn test_scenario_1() {
    let results = RewardCalculatorRunnerTest::for_scenario_1().build_and_run();
    let mut subnet_rates = BTreeMap::new();
    for (_, metrics) in results.results_by_node.iter() {
        for (_, metric) in metrics.daily_metrics.clone().into_iter() {
            subnet_rates.insert((metric.subnet_assigned, metric.day), metric.subnet_assigned_fr);
        }
    }

    let subnet_1_rates = subnet_rates
        .iter()
        .filter(|(subnet, _)| subnet.0 == subnet_id(1))
        .map(|(_, fr)| fr)
        .cloned()
        .collect_vec();

    let expected_subnet_1_fr: Vec<Percent> = vec![dec!(0.5), dec!(0.2), dec!(0.3), dec!(0.2)].into_iter().map_into().collect_vec();
    assert_eq!(subnet_1_rates, expected_subnet_1_fr);

    let subnet_2_rates = subnet_rates
        .iter()
        .filter(|(subnet, _)| subnet.0 == subnet_id(2))
        .map(|(_, fr)| fr)
        .cloned()
        .collect_vec();

    let expected_subnet_2_rates: Vec<Percent> = vec![dec!(0.34), dec!(0.7), dec!(0.4), dec!(0.5)].into_iter().map_into().collect_vec();
    assert_eq!(subnet_2_rates, expected_subnet_2_rates);
}

#[test]
fn test_node_provider_rewards_one_assigned() {
    let nodes_np_1 = vec![node_id(1), node_id(2), node_id(3), node_id(4), node_id(5)];
    let nodes_np_2 = vec![node_id(6), node_id(7), node_id(8)];

    let mut builder = RewardCalculatorRunnerTest::default()
        .with_reward_period(0, 30 * NANOS_PER_DAY)
        .with_rewards_rates("A,B", vec!["type0", "type1", "type3"], 1000, 97)
        // Node Provider 1: node_1 assigned, rest unassigned
        .with_rewardable_nodes(nodes_np_1, "A,B", NodeRewardType::Type1)
        .with_node_metrics(
            node_id(1),
            vec![
                (0, subnet_id(1), 60, 40),
                (NANOS_PER_DAY, subnet_id(1), 80, 20),
                (2 * NANOS_PER_DAY, subnet_id(1), 70, 30),
                (3 * NANOS_PER_DAY, subnet_id(1), 60, 40),
            ],
        );

    // Node Provider 2: all assigned with 0 failure rate this for bringing the subnet failure rate to 0
    for node in nodes_np_2.into_iter() {
        builder = builder.with_node_metrics(
            node,
            vec![
                (0, subnet_id(1), 100, 0),
                (NANOS_PER_DAY, subnet_id(1), 100, 0),
                (2 * NANOS_PER_DAY, subnet_id(1), 100, 0),
                (3 * NANOS_PER_DAY, subnet_id(1), 100, 0),
            ],
        );
    }

    let results = builder.build_and_run();
    assert_eq!(results.rewards_total.get().round(), dec!(421));
}
