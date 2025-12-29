#[derive(Debug, PartialEq, Eq)]
pub enum VersionFillMode {
    FromStart,
    ToEnd,
}

#[derive(PartialEq, Eq)]
pub struct VersionRange {
    from: u64,
    to: u64,
}

impl VersionRange {
    pub fn get_from(&self) -> u64 {
        self.from
    }

    pub fn get_to(&self) -> u64 {
        self.to
    }

    pub fn get_help_text() -> String {
        String::from(
"- Positive numbers are actual version numbers
- Negative numbers are indices relative to the latest version (-1 = latest)
- 0 is not supported
- Version numbers are inclusive."
        )
    }

    pub fn create_from_args(maybe_version: Option<i64>, maybe_version_2: Option<i64>, mode: VersionFillMode, versions_in_registry: &[u64], ) -> anyhow::Result<Self> {
        let length: u64 = versions_in_registry.len() as u64;
        let max_version_u64: u64 = versions_in_registry[length as usize - 1];

        let from_version: u64;
        let to_version: u64;

        match (maybe_version, maybe_version_2) {
            (None, None) => {
                from_version = 1;
                to_version = max_version_u64;

                return Ok(Self { from: from_version, to: to_version });
            }
            (Some(version), None) => {
                let version_u64: u64 = version.abs() as u64;

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
                else if version > 0 {
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
                else {
                    anyhow::bail!("Version 0 is not supported");
                }

                return Ok(Self {
                    from: from_version,
                    to: to_version,
                })
            }
            (Some(version_1), Some(version_2)) => {
                let version_1_u64: u64 = version_1.abs() as u64;
                let version_2_u64: u64 = version_2.abs() as u64;

                if version_1 < 0 && version_2 < 0 {
                    from_version = max_version_u64 - version_1_u64 + 1;
                    to_version = max_version_u64 - version_2_u64 + 1;
                }
                else if version_1 > 0 && version_2 > 0 {
                    from_version = version_1_u64;
                    to_version = version_2_u64;
                }
                else if version_1 * version_1 < 0 {
                    anyhow::bail!("Cannot mix positive version numbers and negative indices");
                }
                else if version_1 == 0 || version_2 == 0 {
                    anyhow::bail!("Version 0 is not supported");
                }
                else {
                    anyhow::bail!("Unsupported combination of version numbers: {}, {}", version_1, version_2);
                }

                return Ok(Self {
                    from: from_version,
                    to: to_version,
                })

            }
            (None, Some(_)) => {
                anyhow::bail!("Only pass second version number is not supported");
            }
        }
    }
}

impl std::fmt::Debug for VersionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "from {} to {}", self.from, self.to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_from_args() {
        struct TestCase {
            description: String,
            input: (Option<i64>, Option<i64>, VersionFillMode, Vec<u64>),
            output: anyhow::Result<VersionRange>,
        }

        let test_cases = vec![
            // None case
            TestCase {
                description: "None input returns full range from 1 to max".to_string(),
                input: (None, None, VersionFillMode::FromStart, vec![1, 2, 3]),
                output: Ok(VersionRange { from: 1, to: 3 }), // from_version=1, to_version=max_version=3
            },
            TestCase {
                description: "None input with ToEnd mode returns full range from 1 to max".to_string(),
                input: (None, None, VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 30 }), // from_version=1, to_version=max_version=30
            },
            // Version 0 error case
            TestCase {
                description: "version 0 is not supported".to_string(),
                input: (Some(0), None, VersionFillMode::FromStart, vec![1, 2, 3]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Negative version out of range
            TestCase {
                description: "negative version number: out of range".to_string(),
                input: (Some(-5), None, VersionFillMode::FromStart, vec![1, 2, 3]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Negative version valid with FromStart
            TestCase {
                description: "negative version -1 with FromStart".to_string(),
                input: (Some(-1), None, VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 30 }), // from_version=1, to_version=max_version_u64-version_u64+1=30-1+1=30
            },
            TestCase {
                description: "negative version -2 with FromStart".to_string(),
                input: (Some(-2), None, VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 29 }), // from_version=1, to_version=max_version_u64-version_u64+1=30-2+1=29
            },
            // Negative version valid with ToEnd
            TestCase {
                description: "negative version -1 with ToEnd".to_string(),
                input: (Some(-1), None, VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 30, to: 30 }), // from_version=max_version_u64-version_u64+1=30-1+1=30, to_version=max_version_u64=30
            },
            TestCase {
                description: "negative version -2 with ToEnd".to_string(),
                input: (Some(-2), None, VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 29, to: 30 }), // from_version=max_version_u64-version_u64+1=30-2+1=29, to_version=max_version_u64=30
            },
            // Positive version out of range
            TestCase {
                description: "positive version number: out of range".to_string(),
                input: (Some(100), None, VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Positive version valid with FromStart
            TestCase {
                description: "positive version 1 with FromStart".to_string(),
                input: (Some(1), None, VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 1 }), // from_version=1, to_version=version_u64=1
            },
            TestCase {
                description: "positive version 2 with FromStart".to_string(),
                input: (Some(2), None, VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 2 }), // from_version=1, to_version=version_u64=2
            },
            // Positive version valid with ToEnd
            TestCase {
                description: "positive version 1 with ToEnd".to_string(),
                input: (Some(1), None, VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 30 }), // from_version=version_u64=1, to_version=max_version_u64=30
            },
            TestCase {
                description: "positive version 2 with ToEnd".to_string(),
                input: (Some(2), None, VersionFillMode::ToEnd, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 2, to: 30 }), // from_version=version_u64=2, to_version=max_version_u64=30
            },
            // Two version arguments - both positive
            TestCase {
                description: "two positive versions: 1 and 2".to_string(),
                input: (Some(1), Some(2), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 1, to: 2 }), // from_version=version_1_u64=1, to_version=version_2_u64=2
            },
            TestCase {
                description: "two positive versions: 2 and 3".to_string(),
                input: (Some(2), Some(3), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 2, to: 3 }), // from_version=version_1_u64=2, to_version=version_2_u64=3
            },
            TestCase {
                description: "two positive versions: 10 and 20".to_string(),
                input: (Some(10), Some(20), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 10, to: 20 }), // from_version=version_1_u64=10, to_version=version_2_u64=20
            },
            // Two version arguments - both negative
            TestCase {
                description: "two negative versions: -1 and -1".to_string(),
                input: (Some(-1), Some(-1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 30, to: 30 }), // from_version=max_version_u64-version_1_u64+1=30-1+1=30, to_version=max_version_u64-version_2_u64+1=30-1+1=30
            },
            TestCase {
                description: "two negative versions: -2 and -1".to_string(),
                input: (Some(-2), Some(-1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 29, to: 30 }), // from_version=max_version_u64-version_1_u64+1=30-2+1=29, to_version=max_version_u64-version_2_u64+1=30-1+1=30
            },
            TestCase {
                description: "two negative versions: -3 and -2".to_string(),
                input: (Some(-3), Some(-2), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: Ok(VersionRange { from: 28, to: 29 }), // from_version=max_version_u64-version_1_u64+1=30-3+1=28, to_version=max_version_u64-version_2_u64+1=30-2+1=29
            },
            // Two version arguments - error cases
            TestCase {
                description: "two versions: version 0 is not supported".to_string(),
                input: (Some(0), Some(1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            TestCase {
                description: "two versions: second version 0 is not supported".to_string(),
                input: (Some(1), Some(0), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            TestCase {
                description: "two versions: mixing positive and negative is not supported".to_string(),
                input: (Some(1), Some(-1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            TestCase {
                description: "two versions: mixing negative and positive is not supported".to_string(),
                input: (Some(-1), Some(1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
            // Two version arguments - only second version provided (error)
            TestCase {
                description: "only second version provided is not supported".to_string(),
                input: (None, Some(1), VersionFillMode::FromStart, vec![10, 20, 30]),
                output: anyhow::Result::Err(anyhow::anyhow!("")),
            },
        ];

        for test_case in test_cases {
            let result = VersionRange::create_from_args(test_case.input.0, test_case.input.1, test_case.input.2, &test_case.input.3);
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