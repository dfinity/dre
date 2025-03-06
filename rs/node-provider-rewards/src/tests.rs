use super::*;
use crate::reward_period::NANOS_PER_DAY;
use ic_base_types::SubnetId;

fn node_id(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet_id(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

fn create_reward_period() -> RewardPeriod {
    RewardPeriod::new(NANOS_PER_DAY, 30 * NANOS_PER_DAY).unwrap()
}

fn create_metrics_by_node() -> BTreeMap<NodeId, Vec<NodeDailyMetrics>> {
    let mut metrics_by_node = BTreeMap::new();
    metrics_by_node.insert(node_id(1), vec![NodeDailyMetrics::new(NANOS_PER_DAY, subnet_id(1), 0, 0)]);
    metrics_by_node
}

fn create_providers_rewardable_nodes() -> BTreeMap<PrincipalId, Vec<NodeId>> {
    let mut providers_rewardable_nodes = BTreeMap::new();
    providers_rewardable_nodes.insert(PrincipalId::new_anonymous(), vec![node_id(1)]);
    providers_rewardable_nodes
}

#[test]
fn test_empty_rewardable_nodes() {
    let reward_period = create_reward_period();
    let result = validate_input(&reward_period, &BTreeMap::new(), &BTreeMap::new());

    assert_eq!(result, Err(RewardCalculationError::EmptyNodes));
}

#[test]
fn test_node_not_in_rewardables() {
    let reward_period = create_reward_period();
    let metrics_by_node = create_metrics_by_node();
    let mut providers_rewardable_nodes = BTreeMap::new();

    providers_rewardable_nodes.insert(PrincipalId::new_anonymous(), vec![node_id(2)]);

    let result = validate_input(&reward_period, &metrics_by_node, &providers_rewardable_nodes);
    assert_eq!(result, Err(RewardCalculationError::NodeNotInRewardables(node_id(1))));
}

#[test]
fn test_metrics_out_of_range() {
    let reward_period = create_reward_period();
    let mut metrics_by_node = create_metrics_by_node();

    let metrics_out_of_range = NodeDailyMetrics::new(0, subnet_id(1), 0, 0);
    metrics_by_node.get_mut(&node_id(1)).unwrap().push(metrics_out_of_range.clone());

    let providers_rewardable_nodes = create_providers_rewardable_nodes();

    let result = validate_input(&reward_period, &metrics_by_node, &providers_rewardable_nodes);

    assert_eq!(
        result,
        Err(RewardCalculationError::NodeMetricsOutOfRange {
            node_id: node_id(1),
            timestamp: *metrics_out_of_range.ts,
            reward_period,
        })
    );
}

#[test]
fn test_same_day_metrics_same_sub() {
    let reward_period = create_reward_period();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeDailyMetrics::new(NANOS_PER_DAY, subnet_id(1), 0, 0));
    let providers_rewardable_nodes = create_providers_rewardable_nodes();

    let result = validate_input(&reward_period, &metrics_by_node, &providers_rewardable_nodes);

    assert_eq!(result, Err(RewardCalculationError::DuplicateMetrics(node_id(1))));
}

#[test]
fn test_same_day_metrics_different_subs() {
    let reward_period = create_reward_period();
    let mut metrics_by_node = create_metrics_by_node();

    metrics_by_node
        .get_mut(&node_id(1))
        .unwrap()
        .push(NodeDailyMetrics::new(NANOS_PER_DAY, subnet_id(2), 0, 0));
    let providers_rewardable_nodes = create_providers_rewardable_nodes();

    let result = validate_input(&reward_period, &metrics_by_node, &providers_rewardable_nodes);

    assert_eq!(result, Ok(()));
}
