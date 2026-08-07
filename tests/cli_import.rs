use jvmfast::cli::{import_pom, CliError};
use jvmfast::import::ImportError;
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-import-{}-{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

const SIMPLE_POM: &str = r#"<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>simple-app</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>org.slf4j</groupId>
      <artifactId>slf4j-api</artifactId>
      <version>2.0.13</version>
    </dependency>
  </dependencies>
</project>
"#;

#[test]
fn import_pom_writes_project_toml_at_the_root_by_default() {
    let root = temp_dir("default-path");
    fs::write(root.join("pom.xml"), SIMPLE_POM).unwrap();

    let summary = import_pom(&root, None).expect("should import");

    assert!(root.join("project.toml").is_file());
    assert!(summary.contains("project.toml written from"));
}

#[test]
fn import_pom_accepts_an_explicit_pom_path() {
    let root = temp_dir("explicit-path");
    let nested = root.join("legacy");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("pom.xml"), SIMPLE_POM).unwrap();

    let summary = import_pom(&root, Some(nested.join("pom.xml").display().to_string()))
        .expect("should import");

    assert!(root.join("project.toml").is_file());
    assert!(summary.contains("project.toml written from"));
}

#[test]
fn import_pom_reports_manual_attention_items_in_the_summary() {
    let root = temp_dir("with-notes");
    fs::write(root.join("pom.xml"), SIMPLE_POM).unwrap();

    let summary = import_pom(&root, None).expect("should import");

    assert!(summary.contains("item(s) need manual attention"));
    assert!(summary.contains("defaulted java-version to \"lts\""));
}

#[test]
fn import_pom_never_overwrites_an_existing_manifest() {
    let root = temp_dir("already-exists");
    fs::write(root.join("pom.xml"), SIMPLE_POM).unwrap();
    fs::write(
        root.join("project.toml"),
        "[project]\nname = \"existing\"\n",
    )
    .unwrap();

    let result = import_pom(&root, None);

    assert!(matches!(
        result,
        Err(CliError::Import(ImportError::ManifestAlreadyExists(_)))
    ));
}
