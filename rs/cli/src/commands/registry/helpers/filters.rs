use serde_json::{json, Value};
use std::str::FromStr;
use regex::Regex;


#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    key: String,
    value: Value,
    comparison: Comparison,
}

impl Filter {
    pub(crate) fn get_help_message() -> String {
        String::from(
            "Filter in `key=value` format. Multiple filters can be provided.
Examples:
    --filter \"rewards_correct!=true\"
    --filter \"node_type=type1\"
    --filter \"subnet_id startswith tdb26\"
    --filter \"node_id contains h5zep\"",
        )
    }

    pub(crate) fn filter_json_value(&self, current: &mut Value) -> bool {
        match current {
            Value::Object(map) => {
                // Check if current object contains key-value pair
                if let Some(v) = map.get(&self.key) {
                    return self.comparison.matches(v, &self.value);
                }

                // Filter nested objects
                map.retain(|_, v| self.filter_json_value(v));

                // If the map is empty consider it doesn't contain the key-value
                !map.is_empty()
            }
            Value::Array(arr) => {
                // Filter entries in the array
                arr.retain_mut(|v| self.filter_json_value(v));

                // If the array is empty consider it doesn't contain the key-value
                !arr.is_empty()
            }
            _ => false, // Since this is a string comparison, non-object and non-array values don't match
        }
    }
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Define the regex pattern for `key comparison value` with optional spaces
        let re = Regex::new(r"^\s*(\w+)\s*\b(.+?)\b\s*(.*)$").unwrap();

        // Capture key, comparison, and value
        if let Some(captures) = re.captures(s) {
            let key = captures[1].to_string();
            let comparison_str = &captures[2];
            let value_str = &captures[3];

            let comparison = Comparison::from_str(comparison_str)?;

            let value = serde_json::from_str(value_str).unwrap_or_else(|_| serde_json::Value::String(value_str.to_string()));

            Ok(Self { key, value, comparison })
        } else {
            anyhow::bail!(
                "Expected format: `key comparison value` (spaces around the comparison are optional, supported comparison: = != > < >= <= re contains startswith endswith), found {}",
                s
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Comparison {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Regex,
    Contains,
    StartsWith,
    EndsWith,
}

impl FromStr for Comparison {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eq" | "=" | "==" => Ok(Comparison::Equal),
            "ne" | "!=" => Ok(Comparison::NotEqual),
            "gt" | ">" => Ok(Comparison::GreaterThan),
            "lt" | "<" => Ok(Comparison::LessThan),
            "ge" | ">=" => Ok(Comparison::GreaterThanOrEqual),
            "le" | "<=" => Ok(Comparison::LessThanOrEqual),
            "regex" | "re" | "matches" | "=~" => Ok(Comparison::Regex),
            "contains" => Ok(Comparison::Contains),
            "startswith" => Ok(Comparison::StartsWith),
            "endswith" => Ok(Comparison::EndsWith),
            _ => anyhow::bail!("Invalid comparison operator: {}", s),
        }
    }
}

impl Comparison {
    fn matches(&self, value: &Value, other: &Value) -> bool {
        match self {
            Comparison::Equal => value == other,
            Comparison::NotEqual => value != other,
            Comparison::GreaterThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() > b.as_f64(),
                (Value::String(a), Value::String(b)) => a > b,
                _ => false,
            },
            Comparison::LessThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() < b.as_f64(),
                (Value::String(a), Value::String(b)) => a < b,
                _ => false,
            },
            Comparison::GreaterThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() >= b.as_f64(),
                (Value::String(a), Value::String(b)) => a >= b,
                _ => false,
            },
            Comparison::LessThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() <= b.as_f64(),
                (Value::String(a), Value::String(b)) => a <= b,
                _ => false,
            },
            Comparison::Regex => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        let re = Regex::new(other).unwrap();
                        re.is_match(s)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Comparison::Contains => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other { s.contains(other) } else { false }
                } else {
                    false
                }
            }
            Comparison::StartsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other { s.starts_with(other) } else { false }
                } else {
                    false
                }
            }
            Comparison::EndsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other { s.ends_with(other) } else { false }
                } else {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        description: String,
        input: String,
        output: anyhow::Result<Filter>,
     }

    #[test]
    fn test_from_str() {
        let test_cases: Vec<TestCase> = vec![
            TestCase {
                description: "valid filter".to_string(),
                input: "node_provider_principal_id startswith wwdbq-xuq".to_string(),
                output: Ok(Filter {
                    key: "node_provider_principal_id".to_string(),
                    value: Value::String("wwdbq-xuq".to_string()),
                    comparison: Comparison::StartsWith,
                }),
            },
            TestCase {
                description: "invalid filter".to_string(),
                input: "not_a_valid_filter".to_string(),
                output: Err(anyhow::anyhow!("")),
            },
        ];

        for test_case in test_cases {
            let result = Filter::from_str(&test_case.input);
            match (&result, &test_case.output) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "{}", test_case.description);
                }
                (Ok(_), Err(_)) => {
                    assert!(false, "{}: expected Err but got Ok", test_case.description);
                }
                (Err(_), Ok(_)) => {
                    assert!(false, "{}: expected Ok but got Err: {}", test_case.description, result.as_ref().unwrap_err());
                }
                (Err(_), Err(_)) => {
                    // Both are errors, test passes
                    assert!(true, "{}", test_case.description);
                }
            }
        }
    }

    #[test]
    fn test_filter_json_value() {
        struct TestCase {
            description: String,
            input: (Filter,Value),
            output: (bool,Value),
        }

        let test_cases: Vec<TestCase> = vec![
            {
                let input_data = json!({});
                let output_data = json!({});

                TestCase {
                    description: "no input, filter not applied to anything".to_string(),
                    input: (Filter::from_str("principal_id startswith wwdbq-xuq").unwrap(), input_data),
                    output: (false, output_data),
                }
            },
            {
                let input_data = json!({"principal_id": "wwdbq-xuq"});
                let output_data = json!({"principal_id": "wwdbq-xuq"});

                TestCase {
                    description: "match: key found, value found".to_string(),
                    input: (Filter::from_str("principal_id startswith wwdbq-xuq").unwrap(), input_data),
                    output: (true, output_data),
                }
             },
             {
                let input_data = json!({"principal_id": "wwdbq-xuq"});
                let output_data = json!({"principal_id": "wwdbq-xuq"});

                TestCase {
                    description: "match: key found, value not found".to_string(),
                    input: (Filter::from_str("principal_id startswith aslkd-lja").unwrap(), input_data),
                    output: (false, output_data),
                }
            },
            {
                let input_data = json!({
                    "node_provider": {
                        "principal_id": "wwdbq-xuq",
                        "number_of_nodes": "10"
                    },
                    "subnets": {
                        "principal_id": "asdkj-iso",
                    }
                });
                let output_data = json!({
                    "node_provider": {
                        "principal_id": "wwdbq-xuq",
                        "number_of_nodes": "10"
                    }
                });

                TestCase {
                    description: "filter matches (key and value), other item is filtered out".to_string(),
                    input: (Filter::from_str("principal_id startswith wwdbq-x").unwrap(), input_data),
                    output: (true, output_data),
                }
            },
            {
                let input_data = json!({
                    "node_provider": {
                        "principal_id": "wwdbq-xuq",
                        "number_of_nodes": "10"
                    },
                    "subnets": {
                        "principal_id": "asdkj-iso",
                    }
                });
                let output_data = json!({});

                TestCase {
                    description: "filter doesn't match, nothing returned".to_string(),
                    input: (Filter::from_str("some_key startswith some_value").unwrap(), input_data),
                    output: (false, output_data),
                }
            },
            {
                let input_data = json!({
                    "node_provider": [
                        {
                            "principal_id": "wwdbq-xuq",
                            "number_of_nodes": "15"
                        },
                        {
                            "principal_id": "asdkj-iso",
                            "number_of_nodes": "10"
                        },
                    ],
                });
                let output_data = json!({
                    "node_provider": [
                        {
                            "principal_id": "wwdbq-xuq",
                            "number_of_nodes": "15"
                        },
                    ],
                });

                TestCase {
                    description: "filter matches (key and value), other item is filtered out".to_string(),
                    input: (Filter::from_str("principal_id startswith wwdbq-x").unwrap(), input_data),
                    output: (true, output_data),
                }
            },
            {
                let input_data = json!({
                    "node_provider": [
                        {
                            "principal_id": "wwdbq-xuq",
                            "number_of_nodes": "15"
                        },
                        {
                            "principal_id": "asdkj-iso",
                            "number_of_nodes": "10"
                        },
                    ],
                });
                let output_data = json!({});

                TestCase {
                    description: "filter matches (key and value), other item is filtered out".to_string(),
                    input: (Filter::from_str("some_key startswith some_value").unwrap(), input_data),
                    output: (false, output_data),
                }
            },
        ];

        for mut test_case in test_cases {
            let result = test_case.input.0.filter_json_value(&mut test_case.input.1);

            match (&result, &test_case.output) {
               (actual, expected) => {
                    assert_eq!(*actual, expected.0, "{}", test_case.description);
                    assert_eq!(test_case.input.1, expected.1, "{}", test_case.description);
                }
            }
        }
    }

    #[test]
    fn test_comparison_matches() {
        struct TestCase {
            description: String,
            input: Comparison,
            output: bool
        }

        let test_cases: Vec<TestCase> = vec![
            {
                TestCase {
                    description: "equal: success".to_string(),
                    input: Comparison::Equal,
                    output: true
                }
            },
            {
                TestCase {
                    description: "equal: failure".to_string(),
                    input: Comparison::Equal,
                    output: false
                }
            },
        ];
        for test_case in test_cases {
            let result = test_case.input.matches(&Value::String("wwdbq-xuq".to_string()), &Value::String("wwdbq".to_string()));
            assert_eq!(result, test_case.output, "{}", test_case.description);
        }

    }
}
