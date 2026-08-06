use jvmfast::cli::{add_dependency, remove_dependency};
use jvmfast::manifest::parse_module;
use std::path::PathBuf;

fn temp_manifest(name: &str, contents: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-edit-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    let path = dir.join("project.toml");
    std::fs::write(&path, contents).expect("should write fixture");
    path
}

const MINIMAL: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";

#[test]
fn add_dependency_creates_dependencies_table_when_absent() {
    let path = temp_manifest("add-new-table", MINIMAL);

    add_dependency(&path, "com.example:demo", "1.0.0").expect("should add");

    let module = parse_module(&path).expect("should still parse");
    assert_eq!(module.declared_dependencies.len(), 1);
    assert_eq!(
        module.declared_dependencies[0].coordinate,
        "com.example:demo"
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn add_dependency_preserves_existing_content_and_comments() {
    let original = format!(
        "{MINIMAL}\n# a comment that should survive\n[dependencies]\n\"com.example:existing\" = \"1.0.0\"\n"
    );
    let path = temp_manifest("add-preserve", &original);

    add_dependency(&path, "com.example:new", "2.0.0").expect("should add");

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("a comment that should survive"));
    assert!(contents.contains("com.example:existing"));
    assert!(contents.contains("com.example:new"));

    let module = parse_module(&path).expect("should still parse");
    assert_eq!(module.declared_dependencies.len(), 2);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn add_dependency_overwrites_existing_version_for_same_coordinate() {
    let original = format!("{MINIMAL}\n[dependencies]\n\"com.example:demo\" = \"1.0.0\"\n");
    let path = temp_manifest("add-overwrite", &original);

    add_dependency(&path, "com.example:demo", "2.0.0").expect("should add");

    let module = parse_module(&path).expect("should still parse");
    assert_eq!(module.declared_dependencies.len(), 1);
    let dep = &module.declared_dependencies[0];
    assert_eq!(
        dep.version_req,
        jvmfast::domain::VersionReq::Explicit("2.0.0".to_string())
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn remove_dependency_deletes_declared_entry() {
    let original = format!("{MINIMAL}\n[dependencies]\n\"com.example:demo\" = \"1.0.0\"\n");
    let path = temp_manifest("remove-existing", &original);

    let removed = remove_dependency(&path, "com.example:demo").expect("should remove");

    assert!(removed);
    let module = parse_module(&path).expect("should still parse");
    assert!(module.declared_dependencies.is_empty());

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn remove_dependency_returns_false_and_leaves_file_untouched_when_absent() {
    let path = temp_manifest("remove-absent", MINIMAL);
    let before = std::fs::read_to_string(&path).unwrap();

    let removed = remove_dependency(&path, "com.example:never-declared").expect("should not error");

    assert!(!removed);
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(before, after);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
