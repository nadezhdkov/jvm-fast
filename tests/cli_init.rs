use jvmfast::cli::{init, CliError};
use jvmfast::init::InitError;
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-init-{}-{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

#[test]
fn init_writes_project_toml_with_explicit_flags() {
    let root = temp_dir("explicit");

    let summary =
        init(&root, Some("my-app".to_string()), Some("21".to_string())).expect("should init");

    assert!(summary.contains("project.toml written at"));
    let manifest = fs::read_to_string(root.join("project.toml")).unwrap();
    assert!(manifest.contains("name = \"my-app\""));
    assert!(manifest.contains("java-version = \"21\""));
}

#[test]
fn init_reports_defaults_applied_in_the_summary() {
    let root = temp_dir("defaults");

    let summary = init(&root, None, None).expect("should init");

    assert!(summary.contains("derived"));
    assert!(summary.contains("lts"));
}

#[test]
fn init_rejects_an_already_initialized_project() {
    let root = temp_dir("already-initialized");
    fs::write(root.join("project.toml"), "[project]\n").unwrap();

    let result = init(&root, None, None);

    assert!(matches!(
        result,
        Err(CliError::Init(InitError::ManifestAlreadyExists(_)))
    ));
}
