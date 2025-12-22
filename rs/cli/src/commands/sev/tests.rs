use crate::commands::registry::helpers::VersionFillMode;
use crate::commands::registry::helpers::VersionRange;

 struct TestCase {
    input: (Option<i64>, VersionFillMode, Vec<u64>),
    output: anyhow::Result<VersionRange>,
 }

 #[test]
fn test_version_range_create_from_arg() {
   //  let test_cases = vec![
   //      TestCase {
   //          input: (Some(-5), VersionFillMode::FromStart, [1, 2, 3]),
   //          output: anyhow::Result::Err(anyhow::anyhow!("")),
   //      },
   //  ];

   //  for test_case in test_cases {
   //      let result = VersionRange::create_from_arg(test_case.input.0, test_case.input.1, test_case.input.2);

   //      match (&result, &test_case.output) {
   //          (Ok(actual), Ok(expected)) => {
   //              assert_eq!(actual, expected, "VersionRange structs should match");
   //          }
   //          (Err(actual_err), Err(expected_err)) => {
   //              // Compare error messages since anyhow::Error doesn't implement PartialEq
   //              assert_eq!(
   //                  actual_err.to_string(),
   //                  expected_err.to_string(),
   //                  "Error messages should match"
   //              );
   //          }
   //          (Ok(_), Err(_)) => {
   //              panic!("Expected error but got Ok result");
   //          }
   //          (Err(_), Ok(_)) => {
   //              panic!("Expected Ok result but got error");
   //          }
   //      }
   //  }
}