#[test]
fn language_version_matches_gositter() {
    // gositter api.go:27, LanguageVersion = 15.
    assert_eq!(gositter::TREE_SITTER_LANGUAGE_VERSION, 15);
}
