use jvmfast::cache::{
    find_artifact, hash_bytes, list_cached_versions, open_index, record_artifact, CacheError,
    CacheStore, CachedArtifact,
};
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cache-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

#[test]
fn artifact_path_uses_two_level_sha256_sharding() {
    let store = CacheStore::new("/cache/root");
    let sha256 = "a1b2c3d4e5f6";

    let path = store.artifact_path(sha256, "jackson-databind-2.17.0.jar");

    assert_eq!(
        path,
        PathBuf::from(
            "/cache/root/artifacts/sha256/a1/b2/a1b2c3d4e5f6/jackson-databind-2.17.0.jar"
        )
    );
}

#[test]
fn write_artifact_rejects_checksum_mismatch() {
    let dir = temp_dir("mismatch");
    let store = CacheStore::new(&dir);

    let result = store.write_artifact(b"hello world", "not-the-real-hash", "hello.txt");

    assert!(matches!(result, Err(CacheError::ChecksumMismatch { .. })));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_artifact_stores_content_at_hash_derived_path() {
    let dir = temp_dir("write");
    let store = CacheStore::new(&dir);
    let contents = b"jar contents go here";
    let sha256 = hash_bytes(contents);

    let path = store
        .write_artifact(contents, &sha256, "demo-1.0.0.jar")
        .expect("should write");

    assert!(path.is_file());
    assert_eq!(std::fs::read(&path).unwrap(), contents);
    assert!(store.is_cached(&sha256, "demo-1.0.0.jar"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_artifact_is_idempotent_when_already_cached() {
    let dir = temp_dir("idempotent");
    let store = CacheStore::new(&dir);
    let contents = b"same content, written twice";
    let sha256 = hash_bytes(contents);

    let first = store
        .write_artifact(contents, &sha256, "demo-1.0.0.jar")
        .expect("should write");
    let second = store
        .write_artifact(contents, &sha256, "demo-1.0.0.jar")
        .expect("should write again without error");

    assert_eq!(first, second);
    assert_eq!(std::fs::read(&second).unwrap(), contents);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_index_creates_schema_when_absent() {
    let dir = temp_dir("index-create");
    let db_path = dir.join("index.db");

    let conn = open_index(&db_path).expect("should open/create index");
    let result = find_artifact(&conn, "com.example:demo", "1.0.0").expect("should query");

    assert!(result.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn record_and_find_artifact_round_trips() {
    let dir = temp_dir("index-roundtrip");
    let db_path = dir.join("index.db");
    let conn = open_index(&db_path).expect("should open index");
    let artifact = CachedArtifact {
        coordinate: "com.example:demo".to_string(),
        version: "1.0.0".to_string(),
        sha256: "abc123".to_string(),
        filename: "demo-1.0.0.jar".to_string(),
    };

    record_artifact(&conn, &artifact).expect("should record");
    let found = find_artifact(&conn, "com.example:demo", "1.0.0")
        .expect("should query")
        .expect("should exist");

    assert_eq!(found, artifact);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn record_artifact_overwrites_existing_entry_for_same_coordinate_and_version() {
    let dir = temp_dir("index-overwrite");
    let db_path = dir.join("index.db");
    let conn = open_index(&db_path).expect("should open index");

    record_artifact(
        &conn,
        &CachedArtifact {
            coordinate: "com.example:demo".to_string(),
            version: "1.0.0".to_string(),
            sha256: "old-hash".to_string(),
            filename: "demo-1.0.0.jar".to_string(),
        },
    )
    .expect("should record");
    record_artifact(
        &conn,
        &CachedArtifact {
            coordinate: "com.example:demo".to_string(),
            version: "1.0.0".to_string(),
            sha256: "new-hash".to_string(),
            filename: "demo-1.0.0.jar".to_string(),
        },
    )
    .expect("should overwrite");

    let found = find_artifact(&conn, "com.example:demo", "1.0.0")
        .expect("should query")
        .expect("should exist");
    assert_eq!(found.sha256, "new-hash");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_cached_versions_returns_only_matching_coordinate_sorted() {
    let dir = temp_dir("index-list");
    let db_path = dir.join("index.db");
    let conn = open_index(&db_path).expect("should open index");

    for version in ["2.17.0", "2.9.0", "2.10.0"] {
        record_artifact(
            &conn,
            &CachedArtifact {
                coordinate: "com.example:demo".to_string(),
                version: version.to_string(),
                sha256: format!("hash-{version}"),
                filename: format!("demo-{version}.jar"),
            },
        )
        .expect("should record");
    }
    record_artifact(
        &conn,
        &CachedArtifact {
            coordinate: "com.example:other".to_string(),
            version: "1.0.0".to_string(),
            sha256: "hash-other".to_string(),
            filename: "other-1.0.0.jar".to_string(),
        },
    )
    .expect("should record");

    let versions = list_cached_versions(&conn, "com.example:demo").expect("should list");

    assert_eq!(versions, vec!["2.10.0", "2.17.0", "2.9.0"]);

    let _ = std::fs::remove_dir_all(&dir);
}
