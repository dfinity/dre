use chrono::NaiveDateTime;
use regex::Regex;

use super::{Index, Release};

pub fn find_latest_release(index: &Index) -> anyhow::Result<Release> {
    let regex = Regex::new(r"rc--(?P<datetime>\d{4}-\d{2}-\d{2}_\d{2}-\d{2})").unwrap();

    let mut mapped: Vec<(Release, NaiveDateTime)> = index
        .releases
        .iter()
        .cloned()
        .filter_map(|release| {
            let captures = match regex.captures(&release.rc_name) {
                Some(captures) => captures,
                None => return None,
            };
            let datetime_str = captures.name("datetime").unwrap().as_str();
            let datetime = match NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d_%H-%M") {
                Ok(val) => val,
                Err(_) => return None,
            };
            Some((release, datetime))
        })
        .collect();

    mapped.sort_by_key(|(_, datetime)| *datetime);
    mapped.reverse();

    match mapped.first() {
        Some((found, _)) => Ok(found.clone()),
        None => Err(anyhow::anyhow!("There aren't any releases that match the criteria")),
    }
}

#[cfg(test)]
mod find_latest_release_tests {
    use super::*;

    #[test]
    fn should_not_find_release_none_match_regex() {
        let index = Index {
            releases: vec![
                Release {
                    rc_name: String::from("bad-name"),
                    versions: Default::default(),
                },
                Release {
                    rc_name: String::from("rc--kind-of-ok_no-no"),
                    versions: Default::default(),
                },
            ],
            ..Default::default()
        };

        let latest = find_latest_release(&index);

        assert!(latest.is_err());
    }

    #[test]
    fn should_return_latest_correct_release() {
        let index = Index {
            releases: vec![
                Release {
                    rc_name: String::from("rc--kind-of-ok_no-no"),
                    ..Default::default()
                },
                Release {
                    rc_name: String::from("rc--2024-03-09_23-01"),
                    ..Default::default()
                },
                Release {
                    rc_name: String::from("rc--2024-03-10_23-01"),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let latest = find_latest_release(&index);

        assert!(latest.is_ok());
        let latest = latest.unwrap();

        assert_eq!(latest.rc_name, String::from("rc--2024-03-10_23-01"))
    }

    #[test]
    fn should_not_return_release_empty_index() {
        let index = Index { ..Default::default() };

        let latest = find_latest_release(&index);

        assert!(latest.is_err())
    }
}
