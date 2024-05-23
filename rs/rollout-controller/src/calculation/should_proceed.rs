use chrono::NaiveDate;

use super::Index;

pub fn should_proceed(index: &Index, today: NaiveDate) -> bool {
    // Check if the plan is paused
    if index.rollout.pause {
        return false;
    }

    // Check if this day should be skipped
    if index.rollout.skip_days.iter().any(|f| f.eq(&today)) {
        return false;
    }

    true
}

#[cfg(test)]
mod should_proceed_tests {
    use std::str::FromStr;

    use chrono::Local;

    use crate::calculation::Rollout;

    use super::*;

    #[test]
    fn should_proceed_not_blocked_and_not_skipped() {
        let index = Index::default();

        assert!(should_proceed(&index, Local::now().to_utc().date_naive()))
    }

    #[test]
    fn should_not_proceed_skipped_day() {
        let day = NaiveDate::from_str("2024-03-11").unwrap();
        let index = Index {
            rollout: Rollout {
                skip_days: vec![day],
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(!should_proceed(&index, day))
    }
}
