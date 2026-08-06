use jvmfast::version::{SemVer, VersionParseError, VersionRequirement};

fn v(s: &str) -> SemVer {
    SemVer::parse(s).expect("valid test version")
}

#[test]
fn caret_major_nonzero_allows_up_to_next_major() {
    let req = VersionRequirement::parse("^2.17.0").unwrap();
    assert!(req.matches(&v("2.17.0")));
    assert!(req.matches(&v("2.99.0")));
    assert!(!req.matches(&v("3.0.0")));
    assert!(!req.matches(&v("2.16.9")));
}

#[test]
fn caret_zero_major_restricts_to_minor() {
    let req = VersionRequirement::parse("^0.17.0").unwrap();
    assert!(req.matches(&v("0.17.0")));
    assert!(req.matches(&v("0.17.5")));
    assert!(!req.matches(&v("0.18.0")));
}

#[test]
fn caret_zero_major_zero_minor_restricts_to_patch() {
    let req = VersionRequirement::parse("^0.0.17").unwrap();
    assert!(req.matches(&v("0.0.17")));
    assert!(!req.matches(&v("0.0.18")));
}

#[test]
fn tilde_restricts_to_minor() {
    let req = VersionRequirement::parse("~2.17.0").unwrap();
    assert!(req.matches(&v("2.17.0")));
    assert!(req.matches(&v("2.17.9")));
    assert!(!req.matches(&v("2.18.0")));
}

#[test]
fn exact_matches_only_the_exact_version() {
    let req = VersionRequirement::parse("2.17.0").unwrap();
    assert!(req.matches(&v("2.17.0")));
    assert!(!req.matches(&v("2.17.1")));
}

#[test]
fn caret_range_never_selects_a_pre_release() {
    let req = VersionRequirement::parse("^2.17.0").unwrap();
    assert!(!req.matches(&v("2.18.0-beta.1")));
}

#[test]
fn tilde_range_never_selects_a_pre_release() {
    let req = VersionRequirement::parse("~2.17.0").unwrap();
    assert!(!req.matches(&v("2.17.9-rc.1")));
}

#[test]
fn pre_release_only_selected_when_declared_explicitly() {
    let req = VersionRequirement::parse("2.18.0-beta.1").unwrap();
    assert!(req.matches(&v("2.18.0-beta.1")));
    assert!(!req.matches(&v("2.18.0")));
}

#[test]
fn select_candidates_filters_and_preserves_order() {
    let req = VersionRequirement::parse("^2.17.0").unwrap();
    let available = vec![v("2.16.0"), v("2.17.0"), v("2.20.5"), v("3.0.0")];
    let selected: Vec<&SemVer> = req.select_candidates(&available);
    assert_eq!(selected, vec![&v("2.17.0"), &v("2.20.5")]);
}

#[test]
fn invalid_version_string_is_a_typed_error() {
    let err = SemVer::parse("not-a-version").unwrap_err();
    assert_eq!(
        err,
        VersionParseError::InvalidVersion("not-a-version".to_string())
    );
}

#[test]
fn version_missing_patch_component_is_rejected() {
    assert!(SemVer::parse("2.17").is_err());
}
