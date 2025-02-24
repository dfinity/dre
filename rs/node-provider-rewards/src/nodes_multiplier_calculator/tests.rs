use super::*;
use crate::reward_period::{TimestampNanos, NANOS_PER_DAY};
use ic_base_types::{PrincipalId, SubnetId};
use itertools::Itertools;
use maplit::btreemap;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

fn subnet_id(index: u64) -> SubnetId {
    SubnetId::from(PrincipalId::new_subnet_test_id(index))
}

// Helper to create node IDs with optional override
fn node_id(index: u64) -> NodeId {
    NodeId::from(PrincipalId::new_node_test_id(index))
}

// Builder for daily failure rates
struct DailyFailureRateBuilder {
    rates: BTreeMap<NodeId, Vec<DailyNodeFailureRate>>,
    default_ts: TimestampNanos,
}

impl DailyFailureRateBuilder {
    fn new(default_ts: TimestampNanos) -> Self {
        Self {
            rates: BTreeMap::new(),
            default_ts,
        }
    }

    fn new_default(default_ts: TimestampNanos) -> Self {
        Self {
            rates: BTreeMap::new(),
            default_ts,
        }
    }

    // Add multiple entries from tuples (subnet_id, node_id, fr)
    fn add_entries(mut self, entries: Vec<(SubnetId, u64, Decimal)>) -> Self {
        for (subnet_id, node_id, fr) in entries {
            self = self.add_entry(subnet_id, node_id, fr);
        }
        self
    }

    // Add single entry with optional timestamp override
    fn add_entry(mut self, subnet_id: SubnetId, node_id: u64, fr: Decimal) -> Self {
        let node_id = NodeId::from(PrincipalId::new_node_test_id(node_id));
        self.rates.entry(node_id).or_default().push(DailyNodeFailureRate {
            ts: self.default_ts,
            value: NodeFailureRate::Defined {
                subnet_assigned: subnet_id,
                value: fr,
            },
        });
        self
    }

    fn build(self) -> BTreeMap<NodeId, Vec<DailyNodeFailureRate>> {
        self.rates
    }
}
// Helper function that returns the test input data.
fn get_test_input() -> BTreeMap<NodeId, Vec<DailyNodeFailureRate>> {
    // Define subnets and their failure rates.
    let subnet_1 = SubnetId::from(PrincipalId::new_user_test_id(100));
    let subnet_1_fr = dec!(0.2);

    let subnet_2 = SubnetId::from(PrincipalId::new_user_test_id(200));
    let subnet_2_fr = dec!(0.4);

    // Define input tuples: (subnet, node identifier, node failure rate).
    let input = btreemap! {
        1 => vec![Some((1, 0.3)), Some((1, 0.3)), None, Some((2, 0.3))],
        2 => vec![Some((2, 0.5)), None, Some((2, 0.1)), Some((1, 0.4))],
        3 => vec![None, Some((1, 0.2)), Some((2, 0.7)), None],
        4 => vec![Some((2, 0.6)), Some((1, 0.1)), None, None],
        5 => vec![None, Some((1, 0.3)), Some((2, 0.8)), Some((1, 0.4))],
    };

    input
        .into_iter()
        .map(|(node_n, rates)| {
            let node_id = NodeId::from(PrincipalId::new_user_test_id(node_n));

            let daily_fr = rates
                .into_iter()
                .enumerate()
                .map(move |(i, fr)| {
                    let ts = i as u64 * NANOS_PER_DAY;

                    let failure_rate = if let Some((subnet_id, fr)) = fr {
                        NodeFailureRate::Defined {
                            subnet_assigned: SubnetId::from(PrincipalId::new_user_test_id(subnet_id)),
                            value: Decimal::from_f64_retain(fr).unwrap(),
                        }
                    } else {
                        NodeFailureRate::Undefined
                    };
                    DailyNodeFailureRate { ts, value: failure_rate }
                })
                .collect_vec();

            (node_id, daily_fr)
        })
        .collect()
}

#[test]
fn test_discount_failure_rate_for_node_1() {
    // Use the helper to create the common test data.
    let daily_failure_rates = get_test_input();

    // Initialize and process the calculator.
    let multiplier = RewardsMultiplierCalculator::new().rewards_multiplier_per_node(daily_failure_rates);

    multiplier.logger.entries.iter().for_each(|entry| {
        println!("{}", entry);
    });

    assert!(false);
}

// #[test]
// fn test_discount_failure_rate_for_node_5() {
//     let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();
//
//     // Expected data for node 5.
//     let node_id = NodeId::from(PrincipalId::new_user_test_id(5));
//     let subnet_2 = input.iter().find(|(_, id, _)| *id == 5).unwrap().0;
//     let node_5_fr = dec!(0.5);
//     let subnet_2_fr = dec!(0.4);
//
//     let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
//     calculator.discount_subnets_failure_rate();
//
//     let node_5_result = calculator.nodes_failure_rates.get(&node_id).unwrap()[0].value.clone();
//
//     assert_eq!(
//         node_5_result,
//         NodeFailureRate::DefinedRelative {
//             subnet_assigned: subnet_2,
//             original_failure_rate: node_5_fr,
//             subnet_failure_rate: subnet_2_fr,
//             value: node_5_fr - subnet_2_fr,
//         }
//     );
// }
//
// #[test]
// fn test_compute_failure_rate_extrapolated() {
//     let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();
//     let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
//     calculator.discount_subnets_failure_rate();
//
//     // Calculate the expected extrapolated failure rate.
//     let total: Decimal = input
//         .iter()
//         .map(|(subnet, _, fr)| {
//             let subnet_fr = if *subnet == SubnetId::from(PrincipalId::new_user_test_id(100)) {
//                 dec!(0.2)
//             } else {
//                 dec!(0.4)
//             };
//             fr - subnet_fr
//         })
//         .sum();
//     let expected_failure_rate_extrapolated = total / Decimal::from(input.len());
//
//     assert_eq!(calculator.compute_extrapolated_failure_rate(), expected_failure_rate_extrapolated);
// }
//
// #[test]
// fn test_replace_undefined_failure_rates() {
//     let (daily_failure_rates, subnets_failure_rate, input) = get_test_input();
//     let mut calculator = NodesMultiplierCalculator::new(daily_failure_rates, &subnets_failure_rate);
//     calculator.discount_subnets_failure_rate();
//
//     // Calculate the expected extrapolated failure rate.
//     let total: Decimal = input
//         .iter()
//         .map(|(subnet, _, fr)| {
//             let subnet_fr = if *subnet == SubnetId::from(PrincipalId::new_user_test_id(100)) {
//                 dec!(0.2)
//             } else {
//                 dec!(0.4)
//             };
//             fr - subnet_fr
//         })
//         .sum();
//     let expected_failure_rate_extrapolated = total / Decimal::from(input.len());
//
//     assert_eq!(calculator.compute_extrapolated_failure_rate(), expected_failure_rate_extrapolated);
// }

//
// fn daily_mocked_failure_rates(metrics: Vec<MockedMetrics>) -> Vec<Decimal> {
//     metrics
//         .into_iter()
//         .flat_map(|mocked_metrics: MockedMetrics| {
//             (0..mocked_metrics.days).map(move |i| {
//                 DailyMetrics::new(
//                     i,
//                     PrincipalId::new_anonymous().into(),
//                     mocked_metrics.proposed_blocks,
//                     mocked_metrics.failed_blocks,
//                 )
//                 .failure_rate
//             })
//         })
//         .collect()
// }
// fn mocked_rewards_table() -> NodeRewardsTable {
//     let mut rates_outer: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
//     let mut rates_inner: BTreeMap<String, NodeRewardRate> = BTreeMap::new();
//     let mut table: BTreeMap<String, NodeRewardRates> = BTreeMap::new();
//
//     let rate_outer = NodeRewardRate {
//         xdr_permyriad_per_node_per_month: 1000,
//         reward_coefficient_percent: Some(97),
//     };
//
//     let rate_inner = NodeRewardRate {
//         xdr_permyriad_per_node_per_month: 1500,
//         reward_coefficient_percent: Some(95),
//     };
//
//     rates_outer.insert("type0".to_string(), rate_outer);
//     rates_outer.insert("type1".to_string(), rate_outer);
//     rates_outer.insert("type3".to_string(), rate_outer);
//
//     rates_inner.insert("type3.1".to_string(), rate_inner);
//
//     table.insert("A,B,C".to_string(), NodeRewardRates { rates: rates_inner });
//     table.insert("A,B".to_string(), NodeRewardRates { rates: rates_outer });
//
//     NodeRewardsTable { table }
// }
//
// #[test]
// fn test_daily_node_metrics() {
//     let subnet1: SubnetId = PrincipalId::new_user_test_id(1).into();
//     let subnet2: SubnetId = PrincipalId::new_user_test_id(2).into();
//
//     let node1 = PrincipalId::new_user_test_id(101);
//     let node2 = PrincipalId::new_user_test_id(102);
//
//     let sub1_day1 = NodeMetricsHistoryResponse {
//         timestamp_nanos: 1,
//         node_metrics: vec![
//             NodeMetrics {
//                 node_id: node1,
//                 num_blocks_proposed_total: 10,
//                 num_block_failures_total: 2,
//             },
//             NodeMetrics {
//                 node_id: node2,
//                 num_blocks_proposed_total: 20,
//                 num_block_failures_total: 5,
//             },
//         ],
//     };
//
//     let sub1_day2 = NodeMetricsHistoryResponse {
//         timestamp_nanos: 2,
//         node_metrics: vec![
//             NodeMetrics {
//                 node_id: node1,
//                 num_blocks_proposed_total: 20,
//                 num_block_failures_total: 12,
//             },
//             NodeMetrics {
//                 node_id: node2,
//                 num_blocks_proposed_total: 25,
//                 num_block_failures_total: 8,
//             },
//         ],
//     };
//
//     // This happens when the node gets redeployed
//     let sub1_day3 = NodeMetricsHistoryResponse {
//         timestamp_nanos: 3,
//         node_metrics: vec![NodeMetrics {
//             node_id: node1,
//             num_blocks_proposed_total: 15,
//             num_block_failures_total: 3,
//         }],
//     };
//
//     // Simulating subnet change
//     let sub2_day3 = NodeMetricsHistoryResponse {
//         timestamp_nanos: 3,
//         node_metrics: vec![NodeMetrics {
//             node_id: node2,
//             num_blocks_proposed_total: 35,
//             num_block_failures_total: 10,
//         }],
//     };
//
//     let input_metrics = BTreeMap::from([
//         (subnet1, vec![sub1_day1, sub1_day2, sub1_day3]),
//         (subnet2, vec![sub2_day3]),
//     ]);
//
//     let result = metrics_in_rewarding_period(input_metrics);
//
//     let metrics_node1 = result.get(&node1.into()).expect("Node1 metrics not found");
//     assert_eq!(metrics_node1[0].subnet_assigned, subnet1);
//     assert_eq!(metrics_node1[0].num_blocks_proposed, 10);
//     assert_eq!(metrics_node1[0].num_blocks_failed, 2);
//
//     assert_eq!(metrics_node1[1].subnet_assigned, subnet1);
//     assert_eq!(metrics_node1[1].num_blocks_proposed, 10);
//     assert_eq!(metrics_node1[1].num_blocks_failed, 10);
//
//     assert_eq!(metrics_node1[2].subnet_assigned, subnet1);
//     assert_eq!(metrics_node1[2].num_blocks_proposed, 15);
//     assert_eq!(metrics_node1[2].num_blocks_failed, 3);
//
//     let metrics_node2 = result.get(&node2.into()).expect("Node2 metrics not found");
//     assert_eq!(metrics_node2[0].subnet_assigned, subnet1);
//     assert_eq!(metrics_node2[0].num_blocks_proposed, 20);
//     assert_eq!(metrics_node2[0].num_blocks_failed, 5);
//
//     assert_eq!(metrics_node2[1].subnet_assigned, subnet1);
//     assert_eq!(metrics_node2[1].num_blocks_proposed, 5);
//     assert_eq!(metrics_node2[1].num_blocks_failed, 3);
//
//     assert_eq!(metrics_node2[2].subnet_assigned, subnet2);
//     assert_eq!(metrics_node2[2].num_blocks_proposed, 10);
//     assert_eq!(metrics_node2[2].num_blocks_failed, 2);
// }
// #[test]
// fn test_rewards_percent() {
//     let mut logger = RewardsLog::default();
//     let daily_fr: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         // Avg. failure rate = 0.4
//         MockedMetrics::new(20, 6, 4),
//         // Avg. failure rate = 0.2
//         MockedMetrics::new(20, 8, 2),
//     ]);
//
//     let result = assigned_multiplier(&mut logger, daily_fr);
//     // Avg. failure rate = 0.3 -> 1 - (0.3-0.1) / (0.6-0.1) * 0.8 = 0.68
//     assert_eq!(result, dec!(0.68));
//
//     let daily_fr: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         // Avg. failure rate = 0.5
//         MockedMetrics::new(1, 5, 5),
//     ]);
//     let result = assigned_multiplier(&mut logger, daily_fr);
//     // Avg. failure rate = 0.5 -> 1 - (0.5-0.1) / (0.6-0.1) * 0.8 = 0.36
//     assert_eq!(result, dec!(0.36));
//
//     let daily_fr: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         // Avg. failure rate = 0.6666666667
//         MockedMetrics::new(1, 200, 400),
//         // Avg. failure rate = 0.8333333333
//         MockedMetrics::new(1, 5, 25), // no penalty
//     ]);
//     let result = assigned_multiplier(&mut logger, daily_fr);
//     // Avg. failure rate = (0.6666666667 + 0.8333333333) / 2 = 0.75
//     // 1 - (0.75-0.1) / (0.6-0.1) * 0.8 = 0.2
//     assert_eq!(result, dec!(0.2));
// }
//
// #[test]
// fn test_rewards_percent_max_reduction() {
//     let mut logger = RewardsLog::default();
//
//     let daily_fr: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         // Avg. failure rate = 0.95
//         MockedMetrics::new(10, 5, 95),
//     ]);
//     let result = assigned_multiplier(&mut logger, daily_fr);
//     assert_eq!(result, dec!(0.2));
// }
//
// #[test]
// fn test_rewards_percent_min_reduction() {
//     let mut logger = RewardsLog::default();
//
//     let daily_fr: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         // Avg. failure rate = 0.1
//         MockedMetrics::new(10, 9, 1),
//     ]);
//     let result = assigned_multiplier(&mut logger, daily_fr);
//     assert_eq!(result, dec!(1));
// }
//
// #[test]
// fn test_same_rewards_percent_if_gaps_no_penalty() {
//     let mut logger = RewardsLog::default();
//     let gap = MockedMetrics::new(1, 10, 0);
//     let daily_fr_mid_gap: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         MockedMetrics::new(1, 6, 4),
//         gap.clone(),
//         MockedMetrics::new(1, 7, 3),
//     ]);
//     let daily_fr_left_gap: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         gap.clone(),
//         MockedMetrics::new(1, 6, 4),
//         MockedMetrics::new(1, 7, 3),
//     ]);
//     let daily_fr_right_gap: Vec<Decimal> = daily_mocked_failure_rates(vec![
//         gap.clone(),
//         MockedMetrics::new(1, 6, 4),
//         MockedMetrics::new(1, 7, 3),
//     ]);
//
//     assert_eq!(
//         assigned_multiplier(&mut logger, daily_fr_mid_gap.clone()),
//         dec!(0.7866666666666666666666666667)
//     );
//
//     assert_eq!(
//         assigned_multiplier(&mut logger, daily_fr_mid_gap.clone()),
//         assigned_multiplier(&mut logger, daily_fr_left_gap.clone())
//     );
//     assert_eq!(
//         assigned_multiplier(&mut logger, daily_fr_right_gap.clone()),
//         assigned_multiplier(&mut logger, daily_fr_left_gap)
//     );
// }
//
// #[test]
// fn test_systematic_fr_calculation() {
//     let subnet1 = SubnetId::new(PrincipalId::new_user_test_id(1));
//
//     let assigned_metrics = from_subnet_daily_metrics(
//         subnet1,
//         vec![
//             (1, vec![0.2, 0.21, 0.1, 0.9, 0.3]), // Ordered: [0.1, 0.2, 0.21, * 0.3, 0.9]
//             (2, vec![0.8, 0.9, 0.5, 0.6, 0.7]),  // Ordered: [0.5, 0.6, 0.7, * 0.8, 0.9]
//             (3, vec![0.5, 0.6, 0.64, 0.8]),      // Ordered: [0.5, 0.6, * 0.64, 0.8]
//             (4, vec![0.5, 0.6]),                 // Ordered: [0.5, * 0.6]
//             (5, vec![0.2, 0.21, 0.1, 0.9, 0.3, 0.23]), // Ordered: [0.1, 0.2, 0.21, 0.23, * 0.3, 0.9]
//         ],
//     );
//
//     let result = systematic_fr_per_subnet(&assigned_metrics);
//
//     let expected: BTreeMap<(SubnetId, TimestampNanos), Decimal> = BTreeMap::from([
//         ((subnet1, 1), dec!(0.3)),
//         ((subnet1, 2), dec!(0.8)),
//         ((subnet1, 3), dec!(0.64)),
//         ((subnet1, 4), dec!(0.6)),
//         ((subnet1, 5), dec!(0.3)),
//     ]);
//
//     assert_eq!(result, expected);
// }
