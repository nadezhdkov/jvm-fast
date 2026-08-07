use jvmfast::pom::parse_pom_xml;
use std::fs;
use std::path::Path;

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/poms")
            .join(name),
    )
    .expect("fixture should exist")
}

#[test]
fn parses_plain_dependencies() {
    let pom = parse_pom_xml(&fixture("simple_dependencies.xml")).expect("should parse");

    assert_eq!(pom.dependencies.len(), 2);
    assert_eq!(pom.dependencies[0].coordinate, "org.slf4j:slf4j-api");
    assert_eq!(pom.dependencies[0].version, "2.0.13");
    assert_eq!(pom.dependencies[0].scope, "compile");
    assert_eq!(
        pom.dependencies[1].coordinate,
        "com.fasterxml.jackson.core:jackson-core"
    );
    assert_eq!(pom.dependencies[1].version, "2.17.0");
    assert_eq!(
        pom.dependencies[1].scope, "",
        "no <scope> tag means the Maven default, an empty string here — never fabricated as \"compile\""
    );
    assert!(pom.managed_dependencies.is_empty());
}

#[test]
fn parses_test_scoped_dependency() {
    let pom = parse_pom_xml(&fixture("test_scoped_dependency.xml")).expect("should parse");

    assert_eq!(pom.dependencies.len(), 1);
    assert_eq!(
        pom.dependencies[0].coordinate,
        "org.junit.jupiter:junit-jupiter"
    );
    assert_eq!(pom.dependencies[0].scope, "test");
}

#[test]
fn parses_dependency_management_including_import() {
    let pom = parse_pom_xml(&fixture("bom_dependency_management.xml")).expect("should parse");

    assert!(pom.dependencies.is_empty());
    assert_eq!(pom.managed_dependencies.len(), 2);
    assert_eq!(
        pom.managed_dependencies[0].coordinate,
        "com.fasterxml.jackson.core:jackson-databind"
    );
    assert!(!pom.managed_dependencies[0].is_bom_import);
    assert_eq!(
        pom.managed_dependencies[1].coordinate,
        "com.example:other-bom"
    );
    assert!(pom.managed_dependencies[1].is_bom_import);
}

#[test]
fn nested_exclusion_tags_do_not_corrupt_the_dependency_coordinate() {
    let pom = parse_pom_xml(&fixture("nested_exclusions.xml")).expect("should parse");

    assert_eq!(pom.dependencies.len(), 1);
    assert_eq!(
        pom.dependencies[0].coordinate,
        "org.apache.httpcomponents:httpclient"
    );
    assert_eq!(pom.dependencies[0].version, "4.5.14");
}

#[test]
fn malformed_xml_is_a_typed_error() {
    let result = parse_pom_xml(&fixture("malformed.xml"));

    assert!(result.is_err());
}
