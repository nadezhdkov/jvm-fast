use jvmfast::import::{import_pom, ImportError};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/import")
            .join(name),
    )
    .expect("fixture should exist")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-import-{}-{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn write_pom(dir: &Path, contents: &str) -> PathBuf {
    let pom_path = dir.join("pom.xml");
    fs::write(&pom_path, contents).expect("should write pom.xml");
    pom_path
}

#[test]
fn imports_a_minimal_pom_defaulting_java_version_to_lts() {
    let dir = temp_dir("simple");
    let pom_path = write_pom(&dir, &fixture("simple_pom.xml"));
    let manifest_path = dir.join("project.toml");

    let report = import_pom(&pom_path, &manifest_path).expect("should import");

    let manifest = fs::read_to_string(&manifest_path).expect("project.toml should exist");
    assert!(manifest.contains("name = \"simple-app\""));
    assert!(manifest.contains("version = \"1.0.0\""));
    assert!(manifest.contains("java-version = \"lts\""));
    assert!(manifest.contains("\"org.slf4j:slf4j-api\" = \"2.0.13\""));

    assert_eq!(report.notes.len(), 1);
    assert!(report.notes[0].contains("defaulted java-version to \"lts\""));
}

#[test]
fn never_overwrites_an_existing_manifest() {
    let dir = temp_dir("already-exists");
    let pom_path = write_pom(&dir, &fixture("simple_pom.xml"));
    let manifest_path = dir.join("project.toml");
    fs::write(&manifest_path, "[project]\nname = \"existing\"\n").unwrap();

    let result = import_pom(&pom_path, &manifest_path);

    assert!(matches!(result, Err(ImportError::ManifestAlreadyExists(_))));
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    assert_eq!(manifest, "[project]\nname = \"existing\"\n");
}

#[test]
fn missing_direct_artifact_id_is_a_typed_error() {
    let dir = temp_dir("no-artifact-id");
    let pom_path = write_pom(&dir, &fixture("no_artifact_id_pom.xml"));
    let manifest_path = dir.join("project.toml");

    let result = import_pom(&pom_path, &manifest_path);

    assert!(matches!(result, Err(ImportError::MissingArtifactId)));
    assert!(!manifest_path.exists());
}

#[test]
fn missing_direct_version_is_a_typed_error() {
    let dir = temp_dir("no-version");
    let pom_path = write_pom(&dir, &fixture("no_version_pom.xml"));
    let manifest_path = dir.join("project.toml");

    let result = import_pom(&pom_path, &manifest_path);

    assert!(matches!(result, Err(ImportError::MissingVersion)));
    assert!(!manifest_path.exists());
}

#[test]
fn imports_a_full_pom_preserving_everything_with_a_direct_equivalent() {
    let dir = temp_dir("full");
    let pom_path = write_pom(&dir, &fixture("full_pom.xml"));
    let manifest_path = dir.join("project.toml");

    let report = import_pom(&pom_path, &manifest_path).expect("should import");
    let manifest = fs::read_to_string(&manifest_path).expect("project.toml should exist");

    // [project] — java-version resolved from maven.compiler.release, not defaulted.
    assert!(manifest.contains("name = \"demo-app\""));
    assert!(manifest.contains("version = \"1.2.3\""));
    assert!(manifest.contains("java-version = \"21\""));
    assert!(manifest.contains("source-encoding = \"UTF-8\""));

    // [dependencies] — plain, property-interpolated, and BOM-managed (`true`).
    assert!(manifest.contains("\"org.slf4j:slf4j-api\" = \"2.0.13\""));
    assert!(manifest.contains("\"com.fasterxml.jackson.core:jackson-databind\" = \"2.17.0\""));
    assert!(manifest.contains("\"com.example:bom-managed-thing\" = true"));
    assert!(manifest.contains("\"org.apache.httpcomponents:httpclient\" = \"4.5.14\""));
    // `[x]` range has a direct equivalence: the pinned exact version.
    assert!(manifest.contains("\"com.example:pinned-range-thing\" = \"3.1.4\""));

    // Skipped, each with a report entry: provided scope, unresolved property, open range.
    assert!(!manifest.contains("servlet-api"));
    assert!(!manifest.contains("unresolved-property-thing"));
    assert!(!manifest.contains("open-range-thing"));

    // [dev-dependencies] — test-scoped dependency.
    assert!(manifest.contains("[dev-dependencies]"));
    assert!(manifest.contains("\"org.junit.jupiter:junit-jupiter\" = \"5.10.2\""));

    // [boms] — the dependencyManagement import, version interpolated.
    assert!(manifest.contains("[boms]"));
    assert!(manifest.contains("\"com.example:internal-bom\" = \"2.17.0\""));

    // [exclusions] — from the httpclient dependency.
    assert!(manifest.contains("[exclusions]"));
    assert!(manifest.contains(
        "\"org.apache.httpcomponents:httpclient\" = [\"commons-logging:commons-logging\"]"
    ));

    // [repositories] — first declared becomes `default`, rest keyed by id.
    assert!(manifest.contains("[repositories]"));
    assert!(manifest.contains("\"default\" = \"https://repo1.maven.org/maven2\""));
    assert!(
        manifest.contains("\"internal\" = \"https://nexus.example.com/repository/maven-releases\"")
    );

    // Report — one entry per omission (provided scope, unresolved property,
    // unresolved range, extra repository, profiles, plugins) — no
    // java-version default note, since maven.compiler.release resolved it.
    assert_eq!(report.notes.len(), 6);
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("servlet-api") && n.contains("provided")));
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("unresolved-property-thing") && n.contains("missing.property")));
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("open-range-thing") && n.contains("[1.0,2.0)")));
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("additional repository") && n.contains("default")));
    assert!(report.notes.iter().any(|n| n.contains("<profiles>")));
    assert!(report.notes.iter().any(|n| n.contains("<plugins>")));
}
