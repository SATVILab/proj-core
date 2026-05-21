use proj::ignore::root;

#[test]
fn test_root_detection() {
    // Find existing root (which contains the actual project VERSION file for proj)
    assert!(root().is_some());
}
