use super::*;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn node(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn daily_subnet_fr(subnets: u64) -> DailySubnetFailureRate {
    PrincipalId::new_subnet_test_id(id).into()
}

fn daily_node_fr(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn generate_nodes_metrics(metrics_by_day: Vec<Vec<(u64, u64, f64)>>) -> BTreeMap<NodeId, Vec<DailyNodeMetrics>> {
    let mut nodes_metrics: BTreeMap<NodeId, Vec<DailyNodeMetrics>> = BTreeMap::new();

    for (day_offset, daily_metrics) in metrics_by_day.into_iter().enumerate() {
        for (subnet_id, node_id, failure_rate) in daily_metrics {
            let daily_metrics = DailyNodeMetrics {
                ts: (day_offset as u64 * NANOS_PER_DAY).into(),
                subnet_assigned: subnet(subnet_id),
                num_blocks_proposed: 0,
                num_blocks_failed: 0,
                failure_rate: Decimal::from_f64(failure_rate).unwrap(),
            };

            nodes_metrics.entry(node(node_id)).or_default().push(daily_metrics);
        }
    }

    nodes_metrics
}

fn generate_input_data() -> (RewardPeriod, BTreeMap<NodeId, Vec<DailyNodeMetrics>>) {
    let reward_period = RewardPeriod {
        start_ts: 0.into(),
        end_ts: (3 * NANOS_PER_DAY).into(),
    };

    // (subnet_id, node_id, failure_rate)
    let daily_node_metrics = vec![
        vec![(1, 1, 0.3), (1, 2, 0.4), (1, 3, 0.5), (2, 5, 0.344), (2, 6, 0.2), (2, 7, 0.2)], // day 0
        vec![(1, 1, 0.2), (1, 2, 0.1), (1, 3, 0.0), (1, 3, 0.6), (2, 4, 0.5), (2, 5, 0.6), (2, 6, 0.7)], // day 1
        vec![(1, 1, 0.1), (1, 2, 0.2), (1, 3, 0.3), (2, 4, 0.4), (2, 5, 0.5)],                // day 2
        vec![(1, 2, 0.2), (2, 3, 0.3), (2, 4, 0.4), (2, 7, 0.5), (2, 6, 0.7)],                // day 3
    ];

    let nodes_metrics = generate_nodes_metrics(daily_node_metrics);

    (reward_period, nodes_metrics)
}

#[test]
fn test_process_subnets_metrics_correctly() {
    // (subnet_id, node_id, failure_rate)
    let (reward_period, daily_metrics_per_node) = generate_input_data();
    let processor = DailyMetricsProcessor {
        daily_metrics_per_node,
        reward_period,
    };
    let subnets_failure_rates = processor.daily_failure_rates_per_subnet();

    assert_eq!(subnets_failure_rates[&subnet(1)][0].value, dec!(0.5));
    assert_eq!(subnets_failure_rates[&subnet(1)][1].value, dec!(0.2));
    assert_eq!(subnets_failure_rates[&subnet(1)][2].value, dec!(0.3));
    assert_eq!(subnets_failure_rates[&subnet(1)][3].value, dec!(0.2));

    assert_eq!(subnets_failure_rates[&subnet(2)][0].value, dec!(0.344));
    assert_eq!(subnets_failure_rates[&subnet(2)][1].value, dec!(0.7));
    assert_eq!(subnets_failure_rates[&subnet(2)][2].value, dec!(0.5));
    assert_eq!(subnets_failure_rates[&subnet(2)][3].value, dec!(0.5));
}

#[test]
fn test_process_defined_failure_rates_correctly() {
    // (subnet_id, node_id, failure_rate)
    let (reward_period, daily_metrics_per_node) = generate_input_data();
    let processor = DailyMetricsProcessor {
        daily_metrics_per_node,
        reward_period,
    };

    let node_failure_rates = processor.daily_failure_rates_in_period(&node(5));

    assert_eq!(
        node_failure_rates[0].value,
        NodeFailureRate::Defined {
            value: dec!(0.344),
            subnet_assigned: subnet(2)
        }
    );
    assert_eq!(
        node_failure_rates[1].value,
        NodeFailureRate::Defined {
            value: dec!(0.6),
            subnet_assigned: subnet(2)
        }
    );
    assert_eq!(
        node_failure_rates[2].value,
        NodeFailureRate::Defined {
            value: dec!(0.5),
            subnet_assigned: subnet(2)
        }
    );

    let node_failure_rates = processor.daily_failure_rates_in_period(&node(5));

    assert_eq!(
        node_failure_rates[0].value,
        NodeFailureRate::Defined {
            value: dec!(0.344),
            subnet_assigned: subnet(2)
        }
    );
    assert_eq!(
        node_failure_rates[1].value,
        NodeFailureRate::Defined {
            value: dec!(0.6),
            subnet_assigned: subnet(2)
        }
    );
    assert_eq!(
        node_failure_rates[2].value,
        NodeFailureRate::Defined {
            value: dec!(0.5),
            subnet_assigned: subnet(2)
        }
    );
}

#[test]
fn test_process_undefined_failure_rates_correctly() {
    // (subnet_id, node_id, failure_rate)
    let (reward_period, daily_metrics_per_node) = generate_input_data();
    let processor = DailyMetricsProcessor {
        daily_metrics_per_node,
        reward_period,
    };

    let node_failure_rates = processor.daily_failure_rates_in_period(&node(0));

    assert_eq!(node_failure_rates[0].value, NodeFailureRate::Undefined);
    assert_eq!(node_failure_rates[1].value, NodeFailureRate::Undefined);
    assert_eq!(node_failure_rates[2].value, NodeFailureRate::Undefined);
    assert_eq!(node_failure_rates[3].value, NodeFailureRate::Undefined);

    let node_failure_rates = processor.daily_failure_rates_in_period(&node(5));

    assert_eq!(node_failure_rates[3].value, NodeFailureRate::Undefined);
}
