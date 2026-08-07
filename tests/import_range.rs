use jvmfast::import::{is_maven_range, translate_maven_range, RangeTranslation};

#[test]
fn recognizes_maven_range_syntax() {
    assert!(is_maven_range("[1.0,2.0)"));
    assert!(is_maven_range("[1.5,)"));
    assert!(is_maven_range("(,2.0]"));
    assert!(is_maven_range("[1.0]"));
}

#[test]
fn plain_versions_and_jvmfast_ranges_are_not_maven_ranges() {
    assert!(!is_maven_range("2.17.0"));
    assert!(!is_maven_range("^2.17.0"));
    assert!(!is_maven_range("~2.17.0"));
}

#[test]
fn single_value_bracket_range_translates_directly_to_a_pinned_version() {
    assert_eq!(
        translate_maven_range("[3.1.4]"),
        RangeTranslation::Direct("3.1.4".to_string())
    );
}

#[test]
fn open_ended_ranges_have_no_direct_equivalence() {
    assert_eq!(
        translate_maven_range("[1.0,2.0)"),
        RangeTranslation::Unresolved
    );
    assert_eq!(
        translate_maven_range("[1.5,)"),
        RangeTranslation::Unresolved
    );
    assert_eq!(
        translate_maven_range("(,2.0]"),
        RangeTranslation::Unresolved
    );
}

#[test]
fn empty_bracket_range_has_no_direct_equivalence() {
    assert_eq!(translate_maven_range("[]"), RangeTranslation::Unresolved);
}
