use super::*;
use crate::metrics::{nodes_failure_rates_in_period, subnets_failure_rates, NodeMetricsDaily};
use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use crate::types::RewardableNode;
use ic_base_types::PrincipalId;
use itertools::Itertools;
use num_traits::FromPrimitive;

fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn ts_day_end(day: u64) -> TimestampNanos {
    *TimestampNanosAtDayEnd::from(day * NANOS_PER_DAY)
}

pub struct FailureRatesBuilder {
    daily_data: BTreeMap<TimestampNanos, Vec<(SubnetId, NodeId, Decimal)>>,
    nodes_daily_metrics: BTreeMap<NodeId, Vec<NodeMetricsDaily>>,
}

impl FailureRatesBuilder {
    pub fn new() -> Self {
        Self {
            daily_data: BTreeMap::new(),
            nodes_daily_metrics: BTreeMap::new(),
        }
    }
    pub fn with_data_next_day(self, data: Vec<(SubnetId, NodeId, f64)>) -> Self {
        let mut daily_data = self.daily_data;
        let processed_data = data
            .into_iter()
            .map(|(subnet_id, node_id, fr)| (subnet_id, node_id, Decimal::from_f64(fr).unwrap()))
            .collect();

        if daily_data.is_empty() {
            daily_data.insert(0, processed_data);
        } else {
            let next_day = daily_data.keys().max().unwrap() + NANOS_PER_DAY;
            daily_data.insert(next_day, processed_data);
        }

        Self { daily_data, ..self }
    }

    pub fn with_all_days_data(self, data: Vec<Vec<(u64, u64, f64)>>) -> Self {
        data.into_iter().fold(self, |builder, day_data| {
            let day_data_processed = day_data
                .into_iter()
                .map(|(subnet_id_u64, node_id_u64, fr)| (subnet_id(subnet_id_u64), node_id(node_id_u64), fr))
                .collect();
            builder.with_data_next_day(day_data_processed)
        })
    }

    pub fn with_node_metrics_subnet_proposed_failed_blocks(self, node_id: NodeId, metrics: Vec<(TimestampNanos, SubnetId, u64, u64)>) -> Self {
        let mut metrics_by_node = self.nodes_daily_metrics;

        metrics_by_node.insert(
            node_id,
            metrics
                .into_iter()
                .map(|(ts, subnet_id, proposed, failed)| NodeMetricsDaily::new(ts, subnet_id, proposed, failed))
                .collect(),
        );

        Self {
            nodes_daily_metrics: metrics_by_node,
            ..self
        }
    }

    pub fn build(
        self,
    ) -> (
        BTreeMap<NodeId, Vec<NodeDailyFailureRate>>,
        BTreeMap<SubnetId, Vec<SubnetDailyFailureRate>>,
    ) {
        let mut metrics_by_node = self.nodes_daily_metrics;
        for (day, entries) in self.daily_data.into_iter() {
            for (subnet, node, rate) in entries {
                let metrics = NodeMetricsDaily {
                    ts: TimestampNanosAtDayEnd::from(day),
                    subnet_assigned: subnet,
                    num_blocks_proposed: 0,
                    num_blocks_failed: 0,
                    failure_rate: rate,
                };
                metrics_by_node.entry(node).or_default().push(metrics);
            }
        }

        let start_ts = metrics_by_node.values().flat_map(|v| v.iter().map(|m| *m.ts)).min().unwrap();
        let end_ts = metrics_by_node.values().flat_map(|v| v.iter().map(|m| *m.ts)).max().unwrap();
        let reward_period = RewardPeriod::new(start_ts, end_ts).unwrap();
        let all_nodes = metrics_by_node.keys().cloned().collect_vec();

        let subnets_failure_rates = subnets_failure_rates(&metrics_by_node);
        let nodes_failure_rates = nodes_failure_rates_in_period(&all_nodes, &reward_period, &metrics_by_node);

        (nodes_failure_rates, subnets_failure_rates)
    }
}

impl Default for FailureRatesBuilder {
    fn default() -> Self {
        // Each inner vector represents one day (days 0 through 3)
        // Tuple: (subnet_id, node_id, failure_rate)
        let input = vec![
            vec![(1, 1, 0.3), (1, 2, 0.4), (1, 3, 0.5), (2, 5, 0.344), (2, 6, 0.2), (2, 7, 0.2)],
            vec![(1, 1, 0.2), (1, 2, 0.1), (1, 3, 0.0), (2, 4, 0.5), (2, 5, 1.0), (2, 6, 0.7), (2, 7, 0.7)],
            vec![(1, 1, 0.1), (1, 2, 0.2), (1, 3, 0.3), (2, 4, 0.4), (2, 5, 1.0), (2, 6, 0.2), (2, 7, 0.1)],
            vec![(1, 2, 0.2), (2, 3, 0.3), (2, 4, 0.4), (2, 6, 0.7), (2, 7, 0.5)],
        ];

        FailureRatesBuilder::new().with_all_days_data(input)
    }
}

// FailureRatesManager tests

#[test]
fn test_calculates_node_failure_rates_correctly() {
    let (nodes_failure_rates, _) = FailureRatesBuilder::new()
        .with_node_metrics_subnet_proposed_failed_blocks(node_id(0), vec![(0, subnet_id(2), 2, 0)])
        .with_node_metrics_subnet_proposed_failed_blocks(node_id(1), vec![(NANOS_PER_DAY, subnet_id(1), 2, 0)])
        .with_node_metrics_subnet_proposed_failed_blocks(node_id(2), vec![])
        .build();

    let node_0_fr = nodes_failure_rates.get(&node_id(0)).unwrap();

    assert_eq!(node_0_fr.len(), 2);
    assert_eq!(
        node_0_fr[0].value,
        NodeFailureRate::Defined {
            subnet_assigned: subnet_id(2),
            value: dec!(0)
        }
    );
    assert_eq!(node_0_fr[1].value, NodeFailureRate::Undefined);

    let node_1_fr = nodes_failure_rates.get(&node_id(1)).unwrap();

    assert_eq!(node_0_fr.len(), 2);
    assert_eq!(node_1_fr[0].value, NodeFailureRate::Undefined);
    assert_eq!(
        node_1_fr[1].value,
        NodeFailureRate::Defined {
            subnet_assigned: subnet_id(1),
            value: dec!(0)
        }
    );

    // Node 2 has no metrics
    let node_2_fr = nodes_failure_rates.get(&node_id(2)).unwrap();

    assert_eq!(node_0_fr.len(), 2);
    assert_eq!(node_2_fr[0].value, NodeFailureRate::Undefined);
}

#[test]
fn test_node_assigned_same_day_multiple_subnets() {
    let (nodes_failure_rates, _) = FailureRatesBuilder::new()
        // Node 1 has been assigned to two subnets on day 0
        .with_node_metrics_subnet_proposed_failed_blocks(node_id(1), vec![(0, subnet_id(1), 2, 0), (0, subnet_id(2), 1, 0)])
        .build();

    let node_1_fr = nodes_failure_rates.get(&node_id(1)).unwrap();

    assert_eq!(node_1_fr.len(), 1);
    // Expected subnet 1 to be selected as the primary subnet because it has the highest number of proposed blocks
    assert_eq!(
        node_1_fr[0].value,
        NodeFailureRate::Defined {
            subnet_assigned: subnet_id(1),
            value: dec!(0)
        }
    );
}

#[test]
fn test_mgr_calculates_subnet_failure_rates_correctly() {
    let (_, subnet_rates) = FailureRatesBuilder::default().build();

    // Subnet 1 expected daily rates: [0.5, 0.2, 0.3, 0.2]
    let subnet_1_rates = subnet_rates.get(&subnet_id(1)).unwrap();

    assert_eq!(subnet_1_rates[0].value, dec!(0.5));
    assert_eq!(subnet_1_rates[1].value, dec!(0.2));
    assert_eq!(subnet_1_rates[2].value, dec!(0.3));
    assert_eq!(subnet_1_rates[3].value, dec!(0.2));

    // Subnet 2 expected daily rates: [0.344, 0.7, 0.5, 0.5]
    let subnet_2_rates = subnet_rates.get(&subnet_id(2)).unwrap();

    assert_eq!(subnet_2_rates[0].value, dec!(0.344));
    assert_eq!(subnet_2_rates[1].value, dec!(0.7));
    assert_eq!(subnet_2_rates[2].value, dec!(0.4));
    assert_eq!(subnet_2_rates[3].value, dec!(0.5));
}

#[test]
fn test_defined_node_failure_rates() {
    let (nodes_failure_rates, _) = FailureRatesBuilder::default().build();
    let node_5_fr = nodes_failure_rates.get(&node_id(5)).unwrap();

    assert_eq!(node_5_fr.len(), 4);
    assert_eq!(
        node_5_fr[0].value,
        NodeFailureRate::Defined {
            subnet_assigned: subnet_id(2),
            value: dec!(0.344),
        }
    );
    assert_eq!(
        node_5_fr[1].value,
        NodeFailureRate::Defined {
            subnet_assigned: subnet_id(2),
            value: dec!(1),
        }
    );
    assert_eq!(
        node_5_fr[2].value,
        NodeFailureRate::Defined {
            subnet_assigned: subnet_id(2),
            value: dec!(1),
        }
    );
    assert_eq!(node_5_fr[3].value, NodeFailureRate::Undefined);
}

// PerformanceMultiplierCalculator tests

impl Default for RewardableNode {
    fn default() -> Self {
        Self {
            node_id: NodeId::from(PrincipalId::default()),
            node_provider_id: PrincipalId::default(),
            region: Default::default(),
            node_type: Default::default(),
        }
    }
}

fn test_rewardable_nodes(nodes: Vec<NodeId>) -> Vec<RewardableNode> {
    nodes
        .iter()
        .map(|node_id| RewardableNode {
            node_id: *node_id,
            ..Default::default()
        })
        .collect()
}

#[test]
fn test_update_relative_failure_rates() {
    let (nodes_failure_rates, subnets_failure_rates) = FailureRatesBuilder::default().build();
    let ctx: PerformanceCalculatorContext<StartPerformanceCalculator> = PerformanceCalculatorContext {
        subnets_fr: &subnets_failure_rates,
        execution_nodes_fr: nodes_failure_rates,
        results_tracker: ResultsTracker::default(),
        _marker: PhantomData,
    };
    let ctx = ctx.next();
    let ctx = ctx.next();

    let node_5_fr = ctx.execution_nodes_fr.get(&node_id(5)).unwrap();

    let mut expected = NodeDailyFailureRate {
        ts: ts_day_end(0),
        value: NodeFailureRate::DefinedRelative {
            subnet_assigned: subnet_id(2),
            original_failure_rate: dec!(0.344),
            subnet_failure_rate: dec!(0.344),
            value: Decimal::ZERO,
        },
    };
    assert_eq!(node_5_fr[0], expected);

    expected.ts = ts_day_end(1);
    expected.value = NodeFailureRate::DefinedRelative {
        subnet_assigned: subnet_id(2),
        original_failure_rate: dec!(1.0),
        subnet_failure_rate: dec!(0.7),
        value: dec!(0.3),
    };

    assert_eq!(node_5_fr[1], expected);

    expected.ts = ts_day_end(2);
    expected.value = NodeFailureRate::DefinedRelative {
        subnet_assigned: subnet_id(2),
        original_failure_rate: dec!(1.0),
        subnet_failure_rate: dec!(0.4),
        value: dec!(0.6),
    };

    assert_eq!(node_5_fr[2], expected);

    expected.ts = ts_day_end(3);
    expected.value = NodeFailureRate::Undefined;

    assert_eq!(node_5_fr[3], expected);
}

#[test]
fn test_compute_failure_rate_extrapolated() {
    let (nodes_failure_rates, subnets_failure_rates) = FailureRatesBuilder::default().build();
    let ctx: PerformanceCalculatorContext<StartPerformanceCalculator> = PerformanceCalculatorContext {
        subnets_fr: &subnets_failure_rates,
        execution_nodes_fr: nodes_failure_rates,
        results_tracker: ResultsTracker::default(),
        _marker: PhantomData,
    };
    let ctx = ctx.next();
    let ctx = ctx.next();
    let ctx = ctx.next();

    let extrapolated_failure_rate = ctx.results_tracker.get_single_result(SingleResult::ExtrapolatedFR);

    // node_1_fr_relative = [0, 0, 0]
    // node_2_fr_relative = [0, 0, 0, 0]
    // node_3_fr_relative = [0, 0, 0, 0]
    // node_4_fr_relative = [0, 0, 0]
    // node_5_fr_relative = [0, 0.3, 0.6] -> avg = 0.3
    // node_6_fr_relative = [0, 0, 0, 0.2] -> avg = 0.05
    // node_7_fr_relative = [0, 0, 0]
    //
    // expected_extrapolated_fr = 0.05

    assert_eq!(extrapolated_failure_rate, &dec!(0.05));
}

#[test]
fn test_calculate_performance_multiplier_by_node() {
    let (nodes_failure_rates, subnets_failure_rates) = FailureRatesBuilder::default().build();
    let rewardable_nodes = test_rewardable_nodes(nodes_failure_rates.keys().cloned().collect_vec());

    let ctx: PerformanceCalculatorContext<StartPerformanceCalculator> = PerformanceCalculatorContext {
        subnets_fr: &subnets_failure_rates,
        execution_nodes_fr: nodes_failure_rates,
        results_tracker: ResultsTracker::default(),
        _marker: PhantomData,
    };
    let ctx = ctx.next();
    let ctx = ctx.next();
    let ctx = ctx.next();
    let ctx = ctx.next();
    let ctx = ctx.next();
    let ctx: PerformanceCalculatorContext<PerformanceMultipliersComputed> = ctx.next();
    let performance_multiplier_by_node = ctx.results_tracker.get_nodes_result(NodeResult::PerformanceMultiplier);

    // node_5_fr = [0, 0.3, 0.6, 0.05] -> avg = 0.2375
    // rewards_reduction: ((0.2375 - 0.1) / (0.6 - 0.1)) * 0.8 = 0.22
    // rewards_multiplier: 1 - 0.22 = 0.78

    for node in rewardable_nodes {
        if node.node_id == node_id(5) {
            assert_eq!(performance_multiplier_by_node[&node.node_id], dec!(0.78));
        } else {
            assert_eq!(performance_multiplier_by_node[&node.node_id], dec!(1));
        }
    }
}
