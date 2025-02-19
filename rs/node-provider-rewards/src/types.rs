use crate::logs::Logger;
use ic_base_types::{NodeId, PrincipalId, SubnetId};
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fmt;

pub type TimestampNanos = u64;

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct DailyMetrics {
    pub ts: u64,
    pub subnet_assigned: SubnetId,
    pub num_blocks_proposed: u64,
    pub num_blocks_failed: u64,
    pub failure_rate: Decimal,
}

impl Default for DailyMetrics {
    fn default() -> Self {
        DailyMetrics {
            ts: 0,
            subnet_assigned: SubnetId::from(PrincipalId::new_anonymous()),
            num_blocks_proposed: 0,
            num_blocks_failed: 0,
            failure_rate: Decimal::ZERO,
        }
    }
}
impl fmt::Display for DailyMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "num_blocks_proposed: {},  num_blocks_failed: {}, failure_rate: {}",
            self.num_blocks_proposed, self.num_blocks_failed, self.failure_rate
        )
    }
}

impl DailyMetrics {
    pub fn new(ts: u64, subnet_assigned: SubnetId, num_blocks_proposed: u64, num_blocks_failed: u64) -> Self {
        let daily_total = num_blocks_proposed + num_blocks_failed;
        let failure_rate = if daily_total == 0 {
            Decimal::ZERO
        } else {
            Decimal::from_f64(num_blocks_failed as f64 / daily_total as f64).unwrap()
        };
        DailyMetrics {
            ts,
            num_blocks_proposed,
            num_blocks_failed,
            subnet_assigned,
            failure_rate,
        }
    }
}
