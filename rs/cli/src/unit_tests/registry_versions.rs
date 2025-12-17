#[test]
fn test_select_versions() {
    use crate::commands::registry::select_versions;

    // Create versions from 1 to 22
    let versions_sorted: Vec<u64> = (1..=22).collect();

    // Test empty range (None means return all versions)
    let result = select_versions(None, &versions_sorted).unwrap();
    let expected: Vec<u64> = (1..=22).collect();
    assert_eq!(result, expected, "empty range (None) should return all versions");

    // Test 8 to 10 (positive version numbers, end-inclusive)
    let result = select_versions(Some(vec![8, 10]), &versions_sorted).unwrap();
    assert_eq!(result, vec![8, 9, 10], "8 to 10 should return [8, 9, 10]");

    // Test 10 to 8 (should be reordered by validate_range, then return 8 to 10)
    let result = select_versions(Some(vec![10, 8]), &versions_sorted).unwrap();
    assert_eq!(result, vec![8, 9, 10], "10 to 8 should be reordered and return [8, 9, 10]");

    // Test -5 -10 (negative indices, should be reordered, then return indices 12 to 17)
    // -10 means index 22-10=12, -5 means index 22-5=17
    // So we get versions at indices 12..=17, which are versions 13 to 18
    let result = select_versions(Some(vec![-5, -10]), &versions_sorted).unwrap();
    assert_eq!(result, vec![13, 14, 15, 16, 17, 18], "-5 -10 should return versions at indices 12 to 17");

    // Test -10 -5 (negative indices, already in order)
    // -10 means index 22-10=12, -5 means index 22-5=17
    let result = select_versions(Some(vec![-10, -5]), &versions_sorted).unwrap();
    assert_eq!(result, vec![13, 14, 15, 16, 17, 18], "-10 -5 should return versions at indices 12 to 17");

    // Test -10 (single negative index, should return from that index to end)
    // -10 means index 22-10=12, so versions from index 12 to 21 (end-inclusive)
    let result = select_versions(Some(vec![-10]), &versions_sorted).unwrap();
    let expected: Vec<u64> = (13..=22).collect();
    assert_eq!(result, expected, "-10 should return versions from index 12 to end");

    // Test 5 (single positive number, should return 1 to 5)
    let result = select_versions(Some(vec![5]), &versions_sorted).unwrap();
    assert_eq!(result, vec![1, 2, 3, 4, 5], "5 should return versions from 1 to 5");

    // Test 0 10 (0 is not supported as a version number)
    let result = select_versions(Some(vec![0, 10]), &versions_sorted);
    assert!(result.is_err(), "0 10 should error because version 0 is not supported");

    // Test 10 10 (same number twice, should return just that version)
    let result = select_versions(Some(vec![10, 10]), &versions_sorted).unwrap();
    assert_eq!(result, vec![10], "10 10 should return [10]");

    // Test negative index out of range (should fail)
    // With 22 versions, -23 would be out of range
    let result = select_versions(Some(vec![-23]), &versions_sorted);
    assert!(result.is_err(), "-23 should error because it's out of range for 22 versions");
    assert!(result.unwrap_err().to_string().contains("out of range"), "Error message should mention 'out of range'");
}
