use jvmfast::init::{init_project, InitError};
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("jvmfast-test-init-{}-{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

#[test]
fn init_writes_a_minimal_manifest_with_explicit_name_and_java_version() {
    let root = temp_dir("explicit");

    let report = init_project(&root, Some("my-app"), Some("21")).expect("should init");
    assert!(report.notes.is_empty());

    let manifest = fs::read_to_string(root.join("project.toml")).unwrap();
    assert!(manifest.contains("name = \"my-app\""));
    assert!(manifest.contains("java-version = \"21\""));
    assert!(manifest.contains("[run]"));
    assert!(manifest.contains("main-class = \"Main\""));
    assert!(manifest.contains("[dependencies]"));
}

#[test]
fn init_derives_the_name_from_the_directory_and_defaults_java_version_to_lts() {
    let root = temp_dir("defaults");

    let report = init_project(&root, None, None).expect("should init");
    assert_eq!(report.notes.len(), 2);
    assert!(report.notes.iter().any(|n| n.contains("derived")));
    assert!(report.notes.iter().any(|n| n.contains("lts")));

    let manifest = fs::read_to_string(root.join("project.toml")).unwrap();
    let dir_name = root.file_name().unwrap().to_str().unwrap();
    assert!(manifest.contains(&format!("name = \"{dir_name}\"")));
    assert!(manifest.contains("java-version = \"lts\""));
}

#[test]
fn init_creates_source_directories_and_a_hello_world_placeholder() {
    let root = temp_dir("scaffold");

    init_project(&root, Some("demo"), Some("21")).expect("should init");

    assert!(root.join("src/main/java").is_dir());
    assert!(root.join("src/test/java").is_dir());
    let main_java = fs::read_to_string(root.join("src/main/java/Main.java")).unwrap();
    assert!(main_java.contains("Hello, World!"));
    assert!(main_java.contains("public class Main"));
}

#[test]
fn init_does_not_overwrite_existing_java_sources() {
    let root = temp_dir("existing-sources");
    fs::create_dir_all(root.join("src/main/java")).unwrap();
    fs::write(root.join("src/main/java/App.java"), "public class App {}\n").unwrap();

    let report = init_project(&root, Some("demo"), Some("21")).expect("should init");

    assert!(report.notes.iter().any(|n| n.contains("already contains")));
    assert!(!root.join("src/main/java/Main.java").exists());
    assert!(root.join("src/main/java/App.java").is_file());

    let manifest = fs::read_to_string(root.join("project.toml")).unwrap();
    assert!(!manifest.contains("[run]"));
}

#[test]
fn init_refuses_to_run_when_project_toml_already_exists() {
    let root = temp_dir("already-initialized");
    fs::write(root.join("project.toml"), "[project]\n").unwrap();

    let result = init_project(&root, Some("demo"), None);

    assert!(matches!(result, Err(InitError::ManifestAlreadyExists(_))));
}

#[test]
fn init_refuses_to_run_when_a_pom_xml_is_present() {
    let root = temp_dir("has-pom");
    fs::write(root.join("pom.xml"), "<project></project>\n").unwrap();

    let result = init_project(&root, Some("demo"), None);

    assert!(matches!(result, Err(InitError::PomXmlDetected(_))));
    assert!(!root.join("project.toml").exists());
}
