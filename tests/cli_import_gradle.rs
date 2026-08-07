use jvmfast::cli::{import_gradle, CliError};
use jvmfast::gradleimport::GradleImportError;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-import-gradle-{}-{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn fixture_project() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gradle/simple-project")
        .display()
        .to_string()
}

#[test]
fn import_gradle_writes_project_toml_at_the_given_root() {
    let root = temp_dir("explicit-path");

    let summary =
        import_gradle(&root, Some(fixture_project())).expect("should import the fixture project");

    assert!(root.join("project.toml").is_file());
    assert!(summary.contains("project.toml written from"));
    assert!(summary.contains("item(s) need manual attention"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn import_gradle_never_overwrites_an_existing_manifest() {
    let root = temp_dir("already-exists");
    fs::write(
        root.join("project.toml"),
        "[project]\nname = \"existing\"\n",
    )
    .unwrap();

    let result = import_gradle(&root, Some(fixture_project()));

    assert!(matches!(
        result,
        Err(CliError::GradleImport(
            GradleImportError::ManifestAlreadyExists(_)
        ))
    ));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn import_gradle_defaults_the_project_argument_to_root() {
    // `project: None` should target `root` itself as the Gradle project to
    // import from — mirrors `import_pom`'s `pom: None` defaulting to
    // `<root>/pom.xml`.
    let root = fixture_project();
    let manifest_dir = temp_dir("default-project-arg");
    // Write the generated manifest into a scratch dir, not the fixture
    // itself, by importing from a copy — but since `import_gradle`'s
    // `manifest_path` is always `root.join("project.toml")`, point `root`
    // straight at a fresh copy of the fixture instead of the checked-in one.
    copy_dir(Path::new(&root), &manifest_dir);

    let summary = jvmfast::cli::import_gradle(&manifest_dir, None).expect("should import");

    assert!(manifest_dir.join("project.toml").is_file());
    assert!(summary.contains("project.toml written from"));

    let _ = fs::remove_dir_all(&manifest_dir);
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let dest_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest_path);
        } else {
            fs::copy(entry.path(), &dest_path).unwrap();
        }
    }
}
