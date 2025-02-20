use super::*;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use rust_decimal::Decimal;
use std::collections::HashMap;

fn node(id: u64) -> NodeId {
    PrincipalId::new_node_test_id(id).into()
}

fn subnet(id: u64) -> SubnetId {
    PrincipalId::new_subnet_test_id(id).into()
}

// Helper to create a RewardPeriod spanning a given number of days.
// We use a fixed start timestamp (2020-01-01 00:00:00 UTC) so that the period is always in the past.
fn create_reward_period(days: u64) -> RewardPeriod {
    // 2020-01-01 00:00:00 in nanoseconds.
    let start_ts: TimestampNanos = 1_577_836_800_000_000_000;
    // Create an unaligned end timestamp halfway into the last day.
    let unaligned_end_ts = start_ts + (days - 1) * NANOS_PER_DAY + NANOS_PER_DAY / 2;
    RewardPeriod::new(start_ts.into(), unaligned_end_ts.into()).unwrap()
}
//
// #[test]
// fn test_daily_failure_rates_per_node() {
//     // Create a reward period spanning 3 days: day0, day1, day2.
//     let rp = RewardPeriod::new(0.into(), (10 * NANOS_PER_DAY).into()).unwrap();
//     let ts0 = rp.start_ts;
//     let ts1 = ts0 + NANOS_PER_DAY;
//     let ts2 = ts0 + 2 * NANOS_PER_DAY;
//
//     // Create two nodes.
//     let n1 = node(1);
//     let n2 = node(2);
//
//     // For n1: metrics on day0 (8 proposed, 2 failed → 0.2) and day2 (5 proposed, 0 failed → 0.0).
//     // For n2: metric on day1 (7 proposed, 3 failed → 0.3).
//     let mut daily_metrics_per_node = HashMap::new();
//     daily_metrics_per_node.insert(
//         n1,
//         vec![DailyNodeMetrics::new(ts0, subnet(1), 8, 2), DailyNodeMetrics::new(ts2, subnet(1), 5, 0)],
//     );
//     daily_metrics_per_node.insert(n2, vec![DailyNodeMetrics::new(ts1, subnet(2), 7, 3)]);
//
//     let processor = MetricsProcessor {
//         daily_metrics_per_node,
//         reward_period: rp.clone(),
//     };
//
//     let results = processor.daily_failure_rates_per_node(&vec![n1, n2]);
//
//     // Every node should have an entry for each day.
//     assert_eq!(results.get(&n1).unwrap().len(), 3);
//     assert_eq!(results.get(&n2).unwrap().len(), 3);
//
//     // -- n1 assertions --
//     let n1_rates = &results[&n1];
//     // Day 0: Defined failure rate 0.2.
//     assert_eq!(n1_rates[0].ts, ts0);
//     match n1_rates[0].value {
//         NodeFailureRate::Defined(rate) => assert_eq!(rate, Decimal::from_f64(0.2).unwrap()),
//         _ => panic!("n1 day0: expected Defined failure rate"),
//     }
//     // Day 1: No metric so Undefined.
//     assert_eq!(n1_rates[1].ts, ts1);
//     assert!(matches!(n1_rates[1].value, NodeFailureRate::Undefined));
//     // Day 2: Defined failure rate 0.0.
//     assert_eq!(n1_rates[2].ts, ts2);
//     match n1_rates[2].value {
//         NodeFailureRate::Defined(rate) => assert_eq!(rate, Decimal::from_f64(0.0).unwrap()),
//         _ => panic!("n1 day2: expected Defined failure rate"),
//     }
//
//     // -- n2 assertions --
//     let n2_rates = &results[&n2];
//     // Day 0: Undefined.
//     assert_eq!(n2_rates[0].ts, ts0);
//     assert!(matches!(n2_rates[0].value, NodeFailureRate::Undefined));
//     // Day 1: Defined failure rate 0.3.
//     assert_eq!(n2_rates[1].ts, ts1);
//     match n2_rates[1].value {
//         NodeFailureRate::Defined(rate) => assert_eq!(rate, Decimal::from_f64(0.3).unwrap()),
//         _ => panic!("n2 day1: expected Defined failure rate"),
//     }
//     // Day 2: Undefined.
//     assert_eq!(n2_rates[2].ts, ts2);
//     assert!(matches!(n2_rates[2].value, NodeFailureRate::Undefined));
// }
//
// #[test]
// fn test_daily_failure_rates_per_subnet() {
//     // Create a reward period spanning 2 days: day0 and day1.
//     let rp = create_reward_period(2);
//     let ts0 = rp.start_ts;
//     let ts1 = ts0 + NANOS_PER_DAY;
//
//     // Two subnets.
//     let s1 = subnet(1);
//     let s2 = subnet(2);
//
//     // Two nodes:
//     // - For n1: day0 in s1 (8 proposed, 2 failed → 0.2) and day1 in s1 (9 proposed, 1 failed → 0.1).
//     // - For n2: day0 in s1 (10 proposed, 6 failed → ≈0.375) and day1 in s2 (5 proposed, 5 failed → 0.5).
//     let n1 = node(1);
//     let n2 = node(2);
//     let mut daily_metrics_per_node = HashMap::new();
//     daily_metrics_per_node.insert(n1, vec![DailyNodeMetrics::new(ts0, s1, 8, 2), DailyNodeMetrics::new(ts1, s1, 9, 1)]);
//     daily_metrics_per_node.insert(n2, vec![DailyNodeMetrics::new(ts0, s1, 10, 6), DailyNodeMetrics::new(ts1, s2, 5, 5)]);
//
//     let processor = MetricsProcessor {
//         daily_metrics_per_node,
//         reward_period: rp.clone(),
//     };
//
//     let subnet_rates = processor.daily_failure_rates_per_subnet();
//
//     // For subnet s1:
//     let s1_rates = subnet_rates.get(&s1).unwrap();
//     // On day0, s1 has two failure rates: 0.2 and ~0.375.
//     // With a 75th percentile, we expect the higher value (~0.375).
//     let day0 = s1_rates.iter().find(|r| r.ts == ts0).unwrap();
//     match day0.value {
//         NodeFailureRate::Defined(rate) => assert_eq!(rate, Decimal::from_f64(0.375).unwrap()),
//         _ => panic!("s1 day0: expected Defined failure rate"),
//     }
//     // On day1, s1 has a single metric with 0.1.
//     let day1 = s1_rates.iter().find(|r| r.ts == ts1).unwrap();
//     match day1.value {
//         NodeFailureRate::Defined(rate) => assert_eq!(rate, Decimal::from_f64(0.1).unwrap()),
//         _ => panic!("s1 day1: expected Defined failure rate"),
//     }
//
//     // For subnet s2: only one metric on day1 with 0.5.
//     let s2_rates = subnet_rates.get(&s2).unwrap();
//     assert_eq!(s2_rates.len(), 1);
//     let s2_day1 = &s2_rates[0];
//     assert_eq!(s2_day1.ts, ts1);
//     match s2_day1.value {
//         NodeFailureRate::Defined(rate) => assert_eq!(rate, Decimal::from_f64(0.5).unwrap()),
//         _ => panic!("s2 day1: expected Defined failure rate"),
//     }
// }
