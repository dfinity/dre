use std::{collections::BTreeMap, fmt::Display};

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

pub fn create_current_release_feature_spec(
    current_release: &Release,
    blessed_versions: Vec<String>,
) -> Result<(String, BTreeMap<String, Vec<String>>), CreateCurrentReleaseFeatureSpecError> {
    let mut current_release_feature_spec: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_release_version = "".to_string();

    for version in &current_release.versions {
        if !blessed_versions.contains(&version.version) {
            return Err(CreateCurrentReleaseFeatureSpecError::VersionNotBlessed {
                rc: current_release.rc_name.to_string(),
                version_name: version.name.to_string(),
                version: version.version.to_string(),
            });
        }

        if version.name.eq(&current_release.rc_name) {
            if current_release_version.is_empty() {
                current_release_version = version.version.to_string();
                continue;
            }

            // Version override attempt. Shouldn't be possible
            return Err(CreateCurrentReleaseFeatureSpecError::VersionSpecifiedTwice {
                rc: current_release.rc_name.to_string(),
                version_name: version.name.to_string(),
            });
        }

        if !version.name.eq(&current_release.rc_name) && version.subnets.is_empty() {
            return Err(CreateCurrentReleaseFeatureSpecError::FeatureBuildNoSubnets {
                rc: current_release.rc_name.to_string(),
                version_name: version.name.to_string(),
            });
        }

        if current_release_feature_spec.contains_key(&version.version) {
            return Err(CreateCurrentReleaseFeatureSpecError::VersionSpecifiedTwice {
                rc: current_release.rc_name.to_string(),
                version_name: version.name.to_string(),
            });
        }

        current_release_feature_spec.insert(version.version.to_string(), version.subnets.clone());
    }

    if current_release_version.is_empty() {
        return Err(CreateCurrentReleaseFeatureSpecError::CurrentVersionNotFound {
            rc: current_release.rc_name.to_string(),
        });
    }

    Ok((current_release_version, current_release_feature_spec))
}

#[derive(PartialEq, Debug)]
pub enum CreateCurrentReleaseFeatureSpecError {
    CurrentVersionNotFound {
        rc: String,
    },
    FeatureBuildNoSubnets {
        rc: String,
        version_name: String,
    },
    VersionNotBlessed {
        rc: String,
        version_name: String,
        version: String,
    },
    VersionSpecifiedTwice {
        rc: String,
        version_name: String,
    },
}

impl Display for CreateCurrentReleaseFeatureSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateCurrentReleaseFeatureSpecError::CurrentVersionNotFound { rc } => f.write_fmt(format_args!(
                "Regular release version not found for release named: {}",
                rc
            )),
            CreateCurrentReleaseFeatureSpecError::FeatureBuildNoSubnets { rc, version_name } => {
                f.write_fmt(format_args!(
                    "Feature build '{}' that is part of rc '{}' doesn't have subnets specified",
                    version_name, rc
                ))
            }
            CreateCurrentReleaseFeatureSpecError::VersionNotBlessed {
                rc,
                version,
                version_name,
            } => f.write_fmt(format_args!(
                "Version '{}', named '{}' that is part of rc '{}' is not blessed",
                version, version_name, rc
            )),
            CreateCurrentReleaseFeatureSpecError::VersionSpecifiedTwice { rc, version_name } => f.write_fmt(
                format_args!("Version '{}' is defined twice within rc named '{}'", version_name, rc),
            ),
        }
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

#[cfg(test)]
mod create_current_release_feature_spec_tests {
    use crate::calculation::Version;

    use super::*;

    #[test]
    fn should_create_map() {
        let blessed_versions = vec![
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
            "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
        let current_release = Release {
            rc_name: "rc--2024-03-10_23-01".to_string(),
            versions: vec![
                Version {
                    name: "rc--2024-03-10_23-01".to_string(),
                    version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01-p2p".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    subnets: vec!["shefu"].iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                },
            ],
        };

        let response = create_current_release_feature_spec(&current_release, blessed_versions);

        assert!(response.is_ok());
        let (current_version, feat_map) = response.unwrap();
        assert_eq!(current_version, "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string());
        assert_eq!(feat_map.len(), 1);

        let (version, subnets) = feat_map.first_key_value().unwrap();
        assert_eq!(version, "85bd56a70e55b2cea75cae6405ae11243e5fdad8");
        assert_eq!(subnets.len(), 1);
        let subnet = subnets.first().unwrap();
        assert_eq!(subnet, "shefu");
    }

    #[test]
    fn shouldnt_create_map_regular_version_not_blessed() {
        let blessed_versions = vec!["85bd56a70e55b2cea75cae6405ae11243e5fdad8"]
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();

        let current_release = Release {
            rc_name: "rc--2024-03-10_23-01".to_string(),
            versions: vec![
                Version {
                    name: "rc--2024-03-10_23-01".to_string(),
                    version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01-p2p".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    subnets: vec!["shefu"].iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                },
            ],
        };

        let response = create_current_release_feature_spec(&current_release, blessed_versions);

        assert!(response.is_err());
        let error = response.err().unwrap();
        assert_eq!(
            error,
            CreateCurrentReleaseFeatureSpecError::VersionNotBlessed {
                rc: "rc--2024-03-10_23-01".to_string(),
                version_name: "rc--2024-03-10_23-01".to_string(),
                version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string()
            }
        )
    }

    #[test]
    fn shouldnt_create_map_version_override() {
        let blessed_versions = vec![
            "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();

        let current_release = Release {
            rc_name: "rc--2024-03-10_23-01".to_string(),
            versions: vec![
                Version {
                    name: "rc--2024-03-10_23-01".to_string(),
                    version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    subnets: vec!["shefu"].iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                },
            ],
        };

        let response = create_current_release_feature_spec(&current_release, blessed_versions);

        assert!(response.is_err());
        let error = response.err().unwrap();
        assert_eq!(
            error,
            CreateCurrentReleaseFeatureSpecError::VersionSpecifiedTwice {
                rc: "rc--2024-03-10_23-01".to_string(),
                version_name: "rc--2024-03-10_23-01".to_string()
            }
        )
    }

    #[test]
    fn shouldnt_create_map_version_override_for_feature_builds() {
        let blessed_versions = vec![
            "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();

        let current_release = Release {
            rc_name: "rc--2024-03-10_23-01".to_string(),
            versions: vec![
                Version {
                    name: "rc--2024-03-10_23-01".to_string(),
                    version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01-p2p".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    subnets: vec!["shefu"].iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01-p2p2".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    subnets: vec!["shefu"].iter().map(|s| s.to_string()).collect(),
                    ..Default::default()
                },
            ],
        };

        let response = create_current_release_feature_spec(&current_release, blessed_versions);

        assert!(response.is_err());
        let error = response.err().unwrap();
        assert_eq!(
            error,
            CreateCurrentReleaseFeatureSpecError::VersionSpecifiedTwice {
                rc: "rc--2024-03-10_23-01".to_string(),
                version_name: "rc--2024-03-10_23-01-p2p2".to_string()
            }
        )
    }

    #[test]
    fn shouldnt_create_map_version_no_subnets_for_feature() {
        let blessed_versions = vec![
            "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();

        let current_release = Release {
            rc_name: "rc--2024-03-10_23-01".to_string(),
            versions: vec![
                Version {
                    name: "rc--2024-03-10_23-01".to_string(),
                    version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01-p2p".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    ..Default::default()
                },
            ],
        };

        let response = create_current_release_feature_spec(&current_release, blessed_versions);

        assert!(response.is_err());
        let error = response.err().unwrap();
        assert_eq!(
            error,
            CreateCurrentReleaseFeatureSpecError::FeatureBuildNoSubnets {
                rc: "rc--2024-03-10_23-01".to_string(),
                version_name: "rc--2024-03-10_23-01-p2p".to_string()
            }
        )
    }

    #[test]
    fn shouldnt_create_map_version_no_regular_build() {
        let blessed_versions = vec![
            "85bd56a70e55b2cea75cae6405ae11243e5fdad8",
            "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();

        let current_release = Release {
            rc_name: "rc--2024-03-10_23-01".to_string(),
            versions: vec![
                Version {
                    name: "rc--2024-03-10_23-01-notregular".to_string(),
                    version: "2e921c9adfc71f3edc96a9eb5d85fc742e7d8a9f".to_string(),
                    subnets: vec!["shefu".to_string()],
                    ..Default::default()
                },
                Version {
                    name: "rc--2024-03-10_23-01-p2p".to_string(),
                    version: "85bd56a70e55b2cea75cae6405ae11243e5fdad8".to_string(),
                    subnets: vec!["qdvhd".to_string()],
                    ..Default::default()
                },
            ],
        };

        let response = create_current_release_feature_spec(&current_release, blessed_versions);

        assert!(response.is_err());
        let error = response.err().unwrap();
        assert_eq!(
            error,
            CreateCurrentReleaseFeatureSpecError::CurrentVersionNotFound {
                rc: "rc--2024-03-10_23-01".to_string(),
            }
        )
    }
}
