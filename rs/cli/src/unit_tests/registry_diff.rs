use colored::Colorize;
use similar::{ChangeTag, TextDiff};

#[test]
fn test_diff_creation() {
    let old = serde_json::json!({
        "a": 1,
        "b": 2
    });
    let new = serde_json::json!({
        "a": 1,
        "b": 3
    });

    let old_str = serde_json::to_string_pretty(&old).unwrap();
    let new_str = serde_json::to_string_pretty(&new).unwrap();

    let diff = TextDiff::from_lines(&old_str, &new_str);
    let mut changes = Vec::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        changes.push(format!("{}{}", sign, change));
    }

    // Verify that we caught the change in "b"
    // The exact formatting depends on serde_json pretty print, but we expect modifications.
    let joined = changes.join("");
    assert!(joined.contains("-  \"b\": 2"));
    assert!(joined.contains("+  \"b\": 3"));
}

#[test]
fn test_diff_identical() {
    let old = serde_json::json!({"a": 1});
    let new = serde_json::json!({"a": 1});

    let old_str = serde_json::to_string_pretty(&old).unwrap();
    let new_str = serde_json::to_string_pretty(&new).unwrap();

    let diff = TextDiff::from_lines(&old_str, &new_str);
    for change in diff.iter_all_changes() {
        assert_eq!(change.tag(), ChangeTag::Equal);
    }
}
