use jvmfast::cache::hash_bytes;
use jvmfast::gradlebridge::extract_bridge_jar;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-gradlebridge-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

#[test]
fn extract_bridge_jar_writes_a_real_jar_to_the_cache() {
    let dir = temp_dir("extract");

    let path = extract_bridge_jar(&dir).expect("should extract embedded bridge jar");

    assert!(path.is_file());
    let bytes = std::fs::read(&path).expect("should read extracted jar");
    // Every zip/jar file (including an empty one) starts with the "PK"
    // local-file-header magic bytes.
    assert_eq!(&bytes[0..2], b"PK");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extract_bridge_jar_is_idempotent_and_content_addressed() {
    let dir = temp_dir("idempotent");

    let first = extract_bridge_jar(&dir).expect("should extract");
    let second = extract_bridge_jar(&dir).expect("should extract again without error");

    assert_eq!(first, second);
    let bytes = std::fs::read(&first).unwrap();
    assert!(first.to_string_lossy().contains(&hash_bytes(&bytes)));

    let _ = std::fs::remove_dir_all(&dir);
}
