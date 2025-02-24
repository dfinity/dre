use super::*;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn make_node_daily_failure_rate_defined(ts: TimestampNanosAtDayEnd, subnet: SubnetId, failure_rate: Decimal) -> NodeDailyFailureRate {
    NodeDailyFailureRate {
        ts: *ts,
        value: NodeFailureRate::Defined {
            subnet_assigned: subnet,
            value: failure_rate,
        },
    }
}

fn generate_node_metrics_data(daily_data: Vec<Vec<(u64, u64, f64)>>) -> BTreeMap<NodeId, Vec<NodeDailyMetrics>> {
    let mut metrics_by_node: BTreeMap<NodeId, Vec<NodeDailyMetrics>> = BTreeMap::new();
    for (day_index, day_data) in daily_data.into_iter().enumerate() {
        for (subnet_id_num, node_id_num, failure_rate) in day_data {
            let metrics = NodeDailyMetrics {
                ts: ((day_index + 1) as u64 * NANOS_PER_DAY).into(),
                subnet_assigned: subnet_id(subnet_id_num),
                num_blocks_proposed: 0,
                num_blocks_failed: 0,
                failure_rate: Decimal::from_f64(failure_rate).unwrap(),
            };
            metrics_by_node.entry(node_id(node_id_num)).or_default().push(metrics);
        }
    }
    metrics_by_node
}

fn create_sample_input_data() -> (RewardPeriod, BTreeMap<NodeId, Vec<NodeDailyMetrics>>) {
    let reward_period = RewardPeriod {
        start_ts: NANOS_PER_DAY.into(),
        end_ts: (4 * NANOS_PER_DAY).into(),
    };

    // (subnet_id, node_id, failure_rate)
    let daily_node_metrics = vec![
        // day 0
        vec![(1, 1, 0.3), (1, 2, 0.4), (1, 3, 0.5), (2, 5, 0.344), (2, 6, 0.2), (2, 7, 0.2)],
        // day 1
        vec![(1, 1, 0.2), (1, 2, 0.1), (1, 3, 0.0), (1, 3, 0.6), (2, 4, 0.5), (2, 5, 0.6), (2, 6, 0.7)],
        // day 2
        vec![(1, 1, 0.1), (1, 2, 0.2), (1, 3, 0.3), (2, 4, 0.4), (2, 5, 0.5)],
        // day 3
        vec![(1, 2, 0.2), (2, 3, 0.3), (2, 4, 0.4), (2, 7, 0.5), (2, 6, 0.7)],
    ];
    let metrics_by_node = generate_node_metrics_data(daily_node_metrics);
    (reward_period, metrics_by_node)
}

#[test]
fn test_calculate_subnet_failure_rates() {
    let (reward_period, metrics_by_node) = create_sample_input_data();
    let aggregator = DailyMetricsAggregator {
        metrics_by_node,
        reward_period,
    };
    let subnet_failure_rates = aggregator.calculate_subnet_failure_rates();

    assert_eq!(subnet_failure_rates[&subnet_id(1)][0].value, dec!(0.5));
    assert_eq!(subnet_failure_rates[&subnet_id(1)][1].value, dec!(0.2));
    assert_eq!(subnet_failure_rates[&subnet_id(1)][2].value, dec!(0.3));
    assert_eq!(subnet_failure_rates[&subnet_id(1)][3].value, dec!(0.2));
    assert_eq!(subnet_failure_rates[&subnet_id(2)][0].value, dec!(0.344));
    assert_eq!(subnet_failure_rates[&subnet_id(2)][1].value, dec!(0.7));
    assert_eq!(subnet_failure_rates[&subnet_id(2)][2].value, dec!(0.5));
    assert_eq!(subnet_failure_rates[&subnet_id(2)][3].value, dec!(0.5));
}

#[test]
fn test_defined_node_failure_rates() {
    let (reward_period, metrics_by_node) = create_sample_input_data();
    let aggregator = DailyMetricsAggregator {
        metrics_by_node,
        reward_period,
    };
    let node_failure_rates = aggregator.calculate_node_failure_rates_for_period(&node_id(5));

    println!("{:?}", node_failure_rates);

    let expected_node_failure_rates = vec![
        make_node_daily_failure_rate_defined((1 * NANOS_PER_DAY).into(), subnet_id(2), dec!(0.344)),
        make_node_daily_failure_rate_defined((2 * NANOS_PER_DAY).into(), subnet_id(2), dec!(0.6)),
        make_node_daily_failure_rate_defined((3 * NANOS_PER_DAY).into(), subnet_id(2), dec!(0.5)),
    ];

    (0..3).for_each(|i| {
        assert_eq!(node_failure_rates[i], expected_node_failure_rates[i]);
    });
}

#[test]
fn test_undefined_node_failure_rates() {
    let (reward_period, metrics_by_node) = create_sample_input_data();
    let aggregator = DailyMetricsAggregator {
        metrics_by_node,
        reward_period,
    };
    let node_failure_rates = aggregator.calculate_node_failure_rates_for_period(&node_id(0));
    for daily_rate in node_failure_rates.iter() {
        assert_eq!(daily_rate.value, NodeFailureRate::Undefined);
    }
    let node_failure_rates = aggregator.calculate_node_failure_rates_for_period(&node_id(5));

    let expected_node_failure_rates = NodeDailyFailureRate {
        ts: *TimestampNanosAtDayEnd::from(4 * NANOS_PER_DAY),
        value: NodeFailureRate::Undefined,
    };

    assert_eq!(node_failure_rates[3], expected_node_failure_rates);
}
