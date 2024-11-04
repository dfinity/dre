use std::fmt;

use crate::types::TimestampNanos;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};

#[derive(Clone)]
pub struct DateTimeRange {
    start_dt: NaiveDateTime,
    end_dt: NaiveDateTime,
}

impl fmt::Display for DateTimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DateTimeRange: start={}, end={}", self.start_dt, self.end_dt)
    }
}

impl DateTimeRange {
    pub fn new(from_ts: TimestampNanos, to_ts: TimestampNanos) -> Self {
        let start_date = Utc.timestamp_nanos(from_ts as i64).date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end_date = Utc.timestamp_nanos(to_ts as i64).date_naive().and_hms_opt(0, 0, 0).unwrap() + Duration::days(1);

        Self {
            start_dt: start_date,
            end_dt: end_date,
        }
    }

    pub fn last_reward_period(current_ts: TimestampNanos) -> Self {
        let current_date = Utc.timestamp_nanos(current_ts as i64).date_naive().and_hms_opt(0, 0, 0).unwrap();
        let (start_date, end_date);

        if current_date.day() >= 14 {
            end_date = NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 14).unwrap();
            let prev_month = current_date - Duration::days(current_date.day() as i64);
            start_date = NaiveDate::from_ymd_opt(prev_month.year(), prev_month.month(), 14).unwrap();
        } else {
            let prev_month = current_date - Duration::days(current_date.day() as i64);
            end_date = NaiveDate::from_ymd_opt(prev_month.year(), prev_month.month(), 14).unwrap();
            let month_before_prev = prev_month - Duration::days(prev_month.day() as i64);
            start_date = NaiveDate::from_ymd_opt(month_before_prev.year(), month_before_prev.month(), 14).unwrap();
        }

        Self {
            start_dt: start_date.and_hms_opt(0, 0, 0).unwrap(),
            end_dt: end_date.and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    pub fn days_between(&self) -> u64 {
        (self.end_dt - self.start_dt).num_days() as u64
    }

    pub fn start_timestamp_nanos(&self) -> TimestampNanos {
        self.start_dt.and_utc().timestamp_nanos_opt().unwrap() as u64
    }

    pub fn end_timestamp_nanos(&self) -> TimestampNanos {
        self.end_dt.and_utc().timestamp_nanos_opt().unwrap() as u64
    }
}

pub fn duration_until_midnight(current_ts: TimestampNanos) -> std::time::Duration {
    let current_dt: NaiveDateTime = Utc.timestamp_nanos(current_ts as i64).naive_utc();
    let mut target_dt: NaiveDateTime = current_dt.date().and_hms_opt(0, 10, 0).unwrap() + Duration::days(1);
    if current_dt >= target_dt {
        target_dt = target_dt + Duration::days(1);
    }

    target_dt.signed_duration_since(current_dt).to_std().expect("Failed to convert duration")
}
