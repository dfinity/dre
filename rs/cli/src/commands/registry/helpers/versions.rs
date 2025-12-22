#[derive(Debug, PartialEq, Eq)]
pub enum VersionFillMode {
    FromStart,
    ToEnd,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VersionRange {
    from: Option<u64>,
    to: Option<u64>,
}

impl VersionRange {
    pub fn create_from_arg(maybe_version: Option<i64>, mode: VersionFillMode, versions_in_registry: &[u64], ) -> anyhow::Result<Self> {
        match maybe_version {
            Some(version) => {
                if version == 0 {
                    anyhow::bail!("Version 0 is not supported");
                }

                let length: u64 = versions_in_registry.len() as u64;
                let version_u64: u64 = version.abs() as u64;
                let max_version_u64: u64 = versions_in_registry[length as usize - 1];

                let mut from_version: u64 = 0;
                let mut to_version: u64 = 0;

                // Handle relative version numbers
                if version < 0 {
                    if version_u64 > length {
                        anyhow::bail!("Relative version number {} is out of range ({} to {})", version, versions_in_registry[0], versions_in_registry[length as usize - 1]);
                    }

                    match mode {
                        VersionFillMode::FromStart => {
                            from_version = 1;
                            to_version = max_version_u64 - version_u64 + 1;

                        }
                        VersionFillMode::ToEnd => {
                            from_version = max_version_u64 - version_u64 + 1;
                            to_version = max_version_u64;
                        }
                    }
                }
                if version > 0 {
                    if version_u64 > max_version_u64 {
                        anyhow::bail!("Version number {} is out of range ({} to {})", version, versions_in_registry[0], max_version_u64);
                    }

                    match mode {
                        VersionFillMode::FromStart => {
                            from_version = 1;
                            to_version = version_u64 ;
                        }
                        VersionFillMode::ToEnd => {
                            from_version = version_u64;
                            to_version = max_version_u64;
                        }
                    }
                }
                return Ok(Self {
                    from: Some(from_version),
                    to: Some(to_version),
                })
            }
            None => {
                let length: u64 = versions_in_registry.len() as u64;
                let max_version_u64: u64 = versions_in_registry[length as usize - 1];

                let from_version: u64 = 1;
                let to_version: u64 = max_version_u64;

                return Ok(Self { from: Some(from_version), to: Some(to_version) });
            }
        }
    }
}

// pub fn create_from_versions(maybe_version_1: Option<i64>, maybe_version_2: Option<i64>, versions_in_registry: &[u64]) -> Self {
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_from_arg() {
        struct TestCase {
            description: String,
            input: (Option<i64>, VersionFillMode, Vec<u64>),
            output: anyhow::Result<VersionRange>,
        }

        let test_cases = vec![
            // None case
            TestCase {
                description: "None input returns full range from 1 to max".to_string(),
                input: (None, VersionFillMode::FromStart, vec![1, 2, 3]),
                output: Ok(VersionRange { from: Some(1), to: Some(3) }), // from_version=1, to_version=max_version=3
            },
            TestCase {
                description: "None input with ToEnd mode returns full range from 1 to max".to_string(),
                input: (None, VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(1), to: Some(30) }), // from_version=1, to_version=max_version=30
            },
            // Version 0 error case
            TestCase {
                description: "version 0 is not supported".to_string(),
                input: (Some(0), VersionFillMode::FromStart, vec![1, 2, 3]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Negative version out of range
            TestCase {
                description: "negative version number: out of range".to_string(),
                input: (Some(-5), VersionFillMode::FromStart, vec![1, 2, 3]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Negative version valid with FromStart
            TestCase {
                description: "negative version -1 with FromStart".to_string(),
                input: (Some(-1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(1), to: Some(30) }), // from_version=1, to_version=max_version_u64-version_u64+1=30-1+1=30
            },
            TestCase {
                description: "negative version -2 with FromStart".to_string(),
                input: (Some(-2), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(1), to: Some(29) }), // from_version=1, to_version=max_version_u64-version_u64+1=30-2+1=29
            },
            // Negative version valid with ToEnd
            TestCase {
                description: "negative version -1 with ToEnd".to_string(),
                input: (Some(-1), VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(30), to: Some(30) }), // from_version=max_version_u64-version_u64+1=30-1+1=30, to_version=max_version_u64=30
            },
            TestCase {
                description: "negative version -2 with ToEnd".to_string(),
                input: (Some(-2), VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(29), to: Some(30) }), // from_version=max_version_u64-version_u64+1=30-2+1=29, to_version=max_version_u64=30
            },
            // Positive version out of range
            TestCase {
                description: "positive version number: out of range".to_string(),
                input: (Some(100), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Positive version valid with FromStart
            TestCase {
                description: "positive version 1 with FromStart".to_string(),
                input: (Some(1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(1), to: Some(1) }), // from_version=1, to_version=version_u64=1
            },
            TestCase {
                description: "positive version 2 with FromStart".to_string(),
                input: (Some(2), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(1), to: Some(2) }), // from_version=1, to_version=version_u64=2
            },
            // Positive version valid with ToEnd
            TestCase {
                description: "positive version 1 with ToEnd".to_string(),
                input: (Some(1), VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(1), to: Some(30) }), // from_version=version_u64=1, to_version=max_version_u64=30
            },
            TestCase {
                description: "positive version 2 with ToEnd".to_string(),
                input: (Some(2), VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: Some(2), to: Some(30) }), // from_version=version_u64=2, to_version=max_version_u64=30
            },
        ];

        for test_case in test_cases {
            let result = VersionRange::create_from_arg(test_case.input.0, test_case.input.1, &test_case.input.2);
            match (&result, &test_case.output) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "{}", test_case.description);
                }
                (Ok(_), Err(_)) => {
                    assert!(false, "{}: expected Err but got Ok", test_case.description);
                }
                (Err(_), Ok(_)) => {
                    assert!(false, "{}: expected Ok but got Err", test_case.description);
                }
                (Err(_), Err(_)) => {
                    assert!(true, "{}", test_case.description);
                }
            }
        }
    }
}