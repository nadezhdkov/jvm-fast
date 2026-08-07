use jvmfast::config::{load_defaults, write_default_java_version};
use std::path::PathBuf;

fn temp_config_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-config-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir.join("config.toml")
}

#[test]
fn load_defaults_returns_default_when_file_absent() {
    let path = temp_config_path("absent");

    let defaults = load_defaults(&path).expect("should not error");

    assert_eq!(defaults.java_version, None);
    assert_eq!(defaults.repository, None);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn write_default_java_version_creates_file_and_parent_dir() {
    let path = temp_config_path("create");

    write_default_java_version(&path, "21").expect("should write");

    let defaults = load_defaults(&path).expect("should read back");
    assert_eq!(defaults.java_version, Some("21".to_string()));

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn write_default_java_version_preserves_other_sections_and_comments() {
    let path = temp_config_path("preserve");
    std::fs::write(
        &path,
        "# comment that should survive\n\
         [defaults]\n\
         repository = \"https://repo1.maven.org/maven2\"\n\
         \n\
         [network]\n\
         max-retries = 5\n",
    )
    .unwrap();

    write_default_java_version(&path, "17").expect("should write");

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("comment that should survive"));
    assert!(contents.contains("max-retries"));
    assert!(contents.contains("repo1.maven.org"));

    let defaults = load_defaults(&path).expect("should read back");
    assert_eq!(defaults.java_version, Some("17".to_string()));
    assert_eq!(
        defaults.repository,
        Some("https://repo1.maven.org/maven2".to_string())
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn write_default_java_version_overwrites_previous_value() {
    let path = temp_config_path("overwrite");

    write_default_java_version(&path, "17").expect("should write");
    write_default_java_version(&path, "21").expect("should overwrite");

    let defaults = load_defaults(&path).expect("should read back");
    assert_eq!(defaults.java_version, Some("21".to_string()));

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
