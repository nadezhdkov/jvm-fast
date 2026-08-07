use jvmfast::gradleimport::{import_gradle, GradleImportError};
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-gradleimport-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn fixture_project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gradle/simple-project")
}

/// Exercises the whole Fase 4 Tooling API pipeline for real: generates the
/// init-script, invokes the embedded bridge jar as a real `java -jar`
/// subprocess, opens a real Tooling API connection to the fixture
/// project's own `gradlew`, resolves real dependencies against real Maven
/// Central (same deliberate, narrow network exception as
/// `tests/cli_build.rs`/`tests/cli_test.rs`'s real-JDK/real-Maven-Central
/// dependencies), and parses the resulting JSON into `project.toml`.
#[test]
fn import_gradle_writes_an_equivalent_project_toml() {
    let manifest_dir = temp_dir("write");
    let manifest_path = manifest_dir.join("project.toml");
    let cache_root = temp_dir("write-cache");

    let report = import_gradle(&fixture_project(), &manifest_path, &cache_root)
        .expect("should import the fixture gradle project");

    let manifest = std::fs::read_to_string(&manifest_path).expect("should read project.toml");
    assert!(manifest.contains("name = \"gradle-import-fixture\""));
    assert!(manifest.contains("version = \"1.2.3\""));
    assert!(manifest.contains("java-version = \"lts\""));
    assert!(manifest.contains("\"org.slf4j:slf4j-api\" = \"2.0.16\""));
    assert!(manifest.contains("[dev-dependencies]"));
    assert!(manifest.contains("\"junit:junit\" = \"4.13.2\""));
    // slf4j-api is a production dependency (compileClasspath/runtimeClasspath) —
    // it must not also be duplicated into [dev-dependencies] just because
    // testCompileClasspath extends implementation and re-lists it too.
    let dev_section = manifest.split("[dev-dependencies]").nth(1).unwrap();
    assert!(!dev_section.contains("slf4j-api"));

    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("defaulted to \"lts\"")));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("highest-version-wins")));

    let _ = std::fs::remove_dir_all(&manifest_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn import_gradle_never_overwrites_an_existing_manifest() {
    let manifest_dir = temp_dir("already-exists");
    let manifest_path = manifest_dir.join("project.toml");
    std::fs::write(&manifest_path, "[project]\nname = \"existing\"\n").unwrap();
    let cache_root = temp_dir("already-exists-cache");

    let result = import_gradle(&fixture_project(), &manifest_path, &cache_root);

    assert!(matches!(
        result,
        Err(GradleImportError::ManifestAlreadyExists(_))
    ));

    let _ = std::fs::remove_dir_all(&manifest_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn import_gradle_rejects_a_directory_without_a_gradlew() {
    let project_dir = temp_dir("no-gradlew");
    let manifest_dir = temp_dir("no-gradlew-manifest");
    let manifest_path = manifest_dir.join("project.toml");
    let cache_root = temp_dir("no-gradlew-cache");

    let result = import_gradle(&project_dir, &manifest_path, &cache_root);

    assert!(matches!(result, Err(GradleImportError::GradlewNotFound(_))));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&manifest_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}
