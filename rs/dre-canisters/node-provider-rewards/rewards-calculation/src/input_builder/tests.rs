use crate::input_builder::{validate_input, NodeMetricsDaily, RewardCalculationError};
use crate::tests::{node_id, subnet_id};
use crate::types::{RewardPeriod, NANOS_PER_DAY};
use ic_base_types::NodeId;
use std::collections::BTreeMap;

fn create_metrics_by_node() -> BTreeMap<NodeId, Vec<NodeMetricsDaily>> {
    let mut metrics_by_node = BTreeMap::new();
    metrics_by_node.insert(node_id(1), vec![NodeMetricsDaily::new(NANOS_PER_DAY, subnet_id(1), 0, 0)]);
    metrics_by_node
}

#[test]
fn test_metrics_out_of_range() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let mut metrics_by_node = create_metrics_by_node();

    let metrics_out_of_range = NodeMetricsDaily::new(0, subnet_id(1), 0, 0);
    metrics_by_node.get_mut(&node_id(1)).unwrap().push(metrics_out_of_range.clone());

    let result = validate_input(&reward_period, &metrics_by_node);

    assert_eq!(
        result,
        Err(RewardCalculationError::NodeMetricsOutOfRange {
            node_id: node_id(1),
            timestamp: metrics_out_of_range.ts.get(),
            reward_period,
        })
    );
}

#[test]
fn test_same_day_metrics_same_sub() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeMetricsDaily::new(NANOS_PER_DAY, subnet_id(1), 0, 0));
    let result = validate_input(&reward_period, &metrics_by_node);

    assert_eq!(result, Err(RewardCalculationError::DuplicateMetrics(node_id(1))));
}

#[test]
fn test_same_day_metrics_different_subs() {
    let reward_period = RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeMetricsDaily::new(NANOS_PER_DAY, subnet_id(2), 0, 0));
    let result = validate_input(&reward_period, &metrics_by_node);

    assert_eq!(result, Ok(()));
}
