use jvmfast::maven::{artifact_filename, artifact_path, artifact_url, MavenLayoutError};

#[test]
fn artifact_path_uses_maven_group_artifact_version_layout() {
    let path = artifact_path(
        "com.fasterxml.jackson.core:jackson-databind",
        "2.17.0",
        "jar",
    )
    .expect("should build path");

    assert_eq!(
        path,
        "com/fasterxml/jackson/core/jackson-databind/2.17.0/jackson-databind-2.17.0.jar"
    );
}

#[test]
fn artifact_url_joins_base_url_and_path_without_double_slash() {
    let url = artifact_url(
        "https://repo1.maven.org/maven2/",
        "com.example:demo",
        "1.0.0",
        "pom",
    )
    .expect("should build url");

    assert_eq!(
        url,
        "https://repo1.maven.org/maven2/com/example/demo/1.0.0/demo-1.0.0.pom"
    );
}

#[test]
fn artifact_filename_is_artifact_dash_version_dot_extension() {
    let filename =
        artifact_filename("com.example:demo", "1.0.0", "jar").expect("should build filename");

    assert_eq!(filename, "demo-1.0.0.jar");
}

#[test]
fn coordinate_without_colon_is_a_typed_error() {
    let result = artifact_path("invalid-coordinate", "1.0.0", "jar");

    assert!(matches!(
        result,
        Err(MavenLayoutError::InvalidCoordinate(_))
    ));
}
