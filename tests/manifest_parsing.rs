use jvmfast::domain::VersionReq;
use jvmfast::manifest::{parse_module, parse_repositories, ManifestError};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn parses_full_manifest_with_all_sections() {
    let module = parse_module(&fixture("valid_full.toml")).expect("should parse");
    assert_eq!(module.name, "licitare-batch-processor");
    assert_eq!(module.declared_dependencies.len(), 2);
    assert_eq!(module.boms.len(), 1);
    assert_eq!(module.exclusions.len(), 1);
}

#[test]
fn parses_minimal_manifest_with_missing_optional_sections() {
    let module = parse_module(&fixture("valid_minimal.toml")).expect("should parse");
    assert_eq!(module.name, "minimal-project");
    assert_eq!(module.declared_dependencies.len(), 1);
    assert!(module.boms.is_empty());
    assert!(module.exclusions.is_empty());
}

#[test]
fn bom_managed_dependency_becomes_version_req_bom_managed() {
    let module = parse_module(&fixture("bom_managed_dependency.toml")).expect("should parse");
    let dep = &module.declared_dependencies[0];
    assert_eq!(
        dep.coordinate,
        "com.fasterxml.jackson.core:jackson-databind"
    );
    assert!(matches!(dep.version_req, VersionReq::BomManaged));
}

#[test]
fn explicit_version_dependency_becomes_version_req_explicit() {
    let module = parse_module(&fixture("valid_minimal.toml")).expect("should parse");
    let dep = &module.declared_dependencies[0];
    match &dep.version_req {
        VersionReq::Explicit(version) => assert_eq!(version, "2.0.13"),
        VersionReq::BomManaged => panic!("expected an explicit version"),
    }
}

#[test]
fn dependency_value_false_is_rejected_as_parse_error() {
    let result = parse_module(&fixture("invalid_dependency_false.toml"));
    assert!(matches!(result, Err(ManifestError::Toml { .. })));
}

#[test]
fn malformed_toml_returns_toml_error() {
    let result = parse_module(&fixture("invalid_toml_syntax.toml"));
    assert!(matches!(result, Err(ManifestError::Toml { .. })));
}

#[test]
fn invalid_coordinate_without_colon_is_rejected() {
    let result = parse_module(&fixture("invalid_coordinate.toml"));
    assert!(matches!(result, Err(ManifestError::InvalidCoordinate(_))));
}

#[test]
fn missing_manifest_file_returns_io_error() {
    let result = parse_module(&fixture("does_not_exist.toml"));
    assert!(matches!(result, Err(ManifestError::Io { .. })));
}

#[test]
fn parse_repositories_reads_declared_named_repositories() {
    let repositories = parse_repositories(&fixture("valid_full.toml")).expect("should parse");
    assert_eq!(
        repositories.get("default").map(String::as_str),
        Some("https://repo1.maven.org/maven2")
    );
    assert_eq!(
        repositories.get("internal").map(String::as_str),
        Some("https://nexus.empresa.com/repository/maven-releases")
    );
}

#[test]
fn parse_repositories_is_empty_when_section_absent() {
    let repositories = parse_repositories(&fixture("valid_minimal.toml")).expect("should parse");
    assert!(repositories.is_empty());
}
