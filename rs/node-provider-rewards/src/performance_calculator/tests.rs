use crate::metrics::{NodeDailyFailureRate, NodeDailyMetrics, NodeFailureRate};
use crate::performance_calculator::{FailureRatesManager, PerformanceMultiplierCalculator};
use crate::reward_period::{RewardPeriod, TimestampNanos, TimestampNanosAtDayEnd, NANOS_PER_DAY};
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use itertools::Itertools;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn ts_day_end(day: u64) -> TimestampNanos {
    *TimestampNanosAtDayEnd::from(day * NANOS_PER_DAY)
}

fn defined_rate(day: u64, subnet: SubnetId, rate: Decimal) -> NodeDailyFailureRate {
    NodeDailyFailureRate {
        ts: ts_day_end(day),
        value: NodeFailureRate::Defined {
            subnet_assigned: subnet,
            value: rate,
        },
    }
}

fn create_input() -> (RewardPeriod, BTreeMap<NodeId, Vec<NodeDailyMetrics>>) {
    let period = RewardPeriod {
        start_ts: 0.into(),
        end_ts: (3 * NANOS_PER_DAY).into(),
    };

    // Each inner vector represents one day (days 1 through 4)
    // Tuple: (subnet_id, node_id, failure_rate)
    // subnet_1_fr = [0.5, 0.2, 0.3, 0.2]
    // subnet_2_fr = [0.344, 0.7, 0.4, 0.5]
    let daily_data = vec![
        vec![(1, 1, 0.3), (1, 2, 0.4), (1, 3, 0.5), (2, 5, 0.344), (2, 6, 0.2), (2, 7, 0.2)],
        vec![(1, 1, 0.2), (1, 2, 0.1), (1, 3, 0.0), (2, 4, 0.5), (2, 5, 1.0), (2, 6, 0.7), (2, 6, 0.7)],
        vec![(1, 1, 0.1), (1, 2, 0.2), (1, 3, 0.3), (2, 4, 0.4), (2, 5, 1.0), (2, 6, 0.2), (2, 7, 0.1)],
        vec![(1, 2, 0.2), (2, 3, 0.3), (2, 4, 0.4), (2, 6, 0.7), (2, 7, 0.5)],
    ];

    let mut metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>> = BTreeMap::new();
    for (day, entries) in daily_data.into_iter().enumerate() {
        for (subnet, node, rate) in entries {
            let metric = NodeDailyMetrics {
                ts: (day as u64 * NANOS_PER_DAY).into(),
                subnet_assigned: subnet_id(subnet),
                num_blocks_proposed: 0,
                num_blocks_failed: 0,
                failure_rate: Decimal::from_f64(rate).unwrap(),
            };
            metrics_by_node.entry(node_id(node)).or_default().push(metric);
        }
    }
    (period, metrics_by_node)
}

fn create_manager() -> FailureRatesManager {
    let (period, metrics) = create_input();
    FailureRatesManager {
        metrics_by_node: metrics,
        reward_period: period,
    }
}

#[test]
fn test_mgr_calculates_subnet_failure_rates_correctly() {
    let mgr = create_manager();
    let subnet_rates = mgr.calculate_subnets_failure_rates();

    // Subnet 1 expected daily rates: [0.5, 0.2, 0.3, 0.2]
    assert_eq!(subnet_rates[&subnet_id(1)][0].value, dec!(0.5));
    assert_eq!(subnet_rates[&subnet_id(1)][1].value, dec!(0.2));
    assert_eq!(subnet_rates[&subnet_id(1)][2].value, dec!(0.3));
    assert_eq!(subnet_rates[&subnet_id(1)][3].value, dec!(0.2));

    // Subnet 2 expected daily rates: [0.344, 0.7, 0.5, 0.5]
    assert_eq!(subnet_rates[&subnet_id(2)][0].value, dec!(0.344));
    assert_eq!(subnet_rates[&subnet_id(2)][1].value, dec!(0.7));
    assert_eq!(subnet_rates[&subnet_id(2)][2].value, dec!(0.4));
    assert_eq!(subnet_rates[&subnet_id(2)][3].value, dec!(0.5));
}

#[test]
fn test_defined_node_failure_rates() {
    let mgr = create_manager();
    let rates = mgr.node_failure_rates_in_period(&node_id(5));
    let expected = vec![
        defined_rate(0, subnet_id(2), dec!(0.344)),
        defined_rate(1, subnet_id(2), dec!(1.0)),
        defined_rate(2, subnet_id(2), dec!(1.0)),
    ];
    for (r, exp) in rates.iter().zip(expected.iter()) {
        assert_eq!(r, exp);
    }
}

#[test]
fn test_undefined_node_failure_rates() {
    let mgr = create_manager();
    let rates = mgr.node_failure_rates_in_period(&node_id(0));
    for rate in &rates {
        assert_eq!(rate.value, NodeFailureRate::Undefined);
    }
    let rates = mgr.node_failure_rates_in_period(&node_id(5));
    let expected = NodeDailyFailureRate {
        ts: ts_day_end(3),
        value: NodeFailureRate::Undefined,
    };
    assert_eq!(rates[3], expected);
}

#[test]
fn test_update_relative_failure_rates() {
    let mgr = create_manager();
    let nodes = mgr.metrics_by_node.keys().cloned().collect_vec();
    let perf_calculator = PerformanceMultiplierCalculator::new(mgr).with_subnets_failure_rates_discount();

    perf_calculator.update_nodes_failure_rates(&nodes);
    perf_calculator.update_relative_failure_rates();
    let nodes_failure_rates = perf_calculator.nodes_failure_rates.take();

    let node_5_fr = nodes_failure_rates.get(&node_id(5)).unwrap();

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
    let mgr = create_manager();
    let nodes = mgr.metrics_by_node.keys().cloned().collect_vec();
    let perf_calculator = PerformanceMultiplierCalculator::new(mgr).with_subnets_failure_rates_discount();

    perf_calculator.update_nodes_failure_rates(&nodes);
    perf_calculator.update_relative_failure_rates();
    let extrapolated_failure_rate = perf_calculator.calculate_extrapolated_failure_rate();

    // node_1_fr_relative = [0, 0, 0]
    // node_2_fr_relative = [0, 0, 0, 0]
    // node_3_fr_relative = [0, 0, 0, 0]
    // node_4_fr_relative = [0, 0, 0]
    // node_5_fr_relative = [0, 0.3, 0.6] -> avg = 0.3
    // node_6_fr_relative = [0, 0, 0, 0.2] -> avg = 0.05
    // node_7_fr_relative = [0, 0, 0]
    //
    // expected_extrapolated_fr = 0.05

    assert_eq!(extrapolated_failure_rate, dec!(0.05));
}

#[test]
fn test_calculate_performance_multiplier_by_node() {
    let mgr = create_manager();
    let nodes = mgr.metrics_by_node.keys().cloned().collect_vec();
    let perf_calculator = PerformanceMultiplierCalculator::new(mgr).with_subnets_failure_rates_discount();

    perf_calculator.update_nodes_failure_rates(&nodes);
    perf_calculator.update_relative_failure_rates();
    let extrapolated_failure_rate = perf_calculator.calculate_extrapolated_failure_rate();
    perf_calculator.fill_undefined_failure_rates(extrapolated_failure_rate);
    let node_average_failure_rates = perf_calculator.calculate_average_failure_rate_by_node();
    let performance_multiplier_by_node = perf_calculator.calculate_performance_multiplier_by_node(&node_average_failure_rates);

    // node_5_fr = [0, 0.3, 0.6, 0.05] -> avg = 0.2375
    // rewards_reduction: ((0.2375 - 0.1) / (0.6 - 0.1)) * 0.8 = 0.22
    // rewards_multiplier: 1 - 0.22 = 0.78

    (1..8).for_each(|idx| {
        if idx == 5 {
            assert_eq!(performance_multiplier_by_node[&node_id(idx)], dec!(0.78));
        } else {
            assert_eq!(performance_multiplier_by_node[&node_id(idx)], dec!(1));
        }
    })
}
