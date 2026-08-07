use jvmfast::domain::{
    DependencyGraph, EdgeKind, GraphEdge, LockedPackage, LockedRequest, Lockfile, MediationReason,
    NodeId, ResolvedNode, VersionRequest,
};
use jvmfast::lockfile::{
    build_lockfile, compute_manifest_hash, is_lockfile_valid, read_lockfile, write_lockfile,
    LockfileError,
};
use std::collections::HashMap;
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("jvmfast-test-{}-{}", std::process::id(), name))
}

#[test]
fn compute_manifest_hash_is_deterministic_and_content_sensitive() {
    let a1 = compute_manifest_hash(["[project]\nname = \"a\""]);
    let a2 = compute_manifest_hash(["[project]\nname = \"a\""]);
    let b = compute_manifest_hash(["[project]\nname = \"b\""]);

    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert!(a1.starts_with("sha256:"));
}

#[test]
fn is_lockfile_valid_checks_manifest_hash() {
    let lockfile = Lockfile {
        version: 1,
        manifest_hash: "sha256:abc".to_string(),
        java_version: "21".to_string(),
        packages: Vec::new(),
        requests: Vec::new(),
    };

    assert!(is_lockfile_valid(&lockfile, "sha256:abc"));
    assert!(!is_lockfile_valid(&lockfile, "sha256:def"));
}

#[test]
fn lockfile_round_trips_through_toml_with_documented_field_names() {
    let lockfile = Lockfile {
        version: 1,
        manifest_hash: "sha256:e3f8a1".to_string(),
        java_version: "21".to_string(),
        packages: vec![LockedPackage {
            name: "com.fasterxml.jackson.core:jackson-databind".to_string(),
            version: "2.17.0".to_string(),
            sha256: "a1b2c3".to_string(),
            resolved_from: "default".to_string(),
            dependencies: vec!["com.fasterxml.jackson.core:jackson-core@2.17.0".to_string()],
        }],
        requests: vec![LockedRequest {
            module: "core".to_string(),
            name: "com.exemplo:commons".to_string(),
            version: "1.8.0".to_string(),
            depth: 1,
        }],
    };

    let serialized = toml::to_string_pretty(&lockfile).expect("should serialize");
    assert!(serialized.contains("manifest-hash"));
    assert!(serialized.contains("java-version = \"21\""));
    assert!(serialized.contains("resolved-from"));
    assert!(serialized.contains("[[package]]"));
    assert!(serialized.contains("[[request]]"));

    let round_tripped: Lockfile = toml::from_str(&serialized).expect("should deserialize");
    assert_eq!(round_tripped, lockfile);
}

fn sample_graph() -> DependencyGraph {
    let mut nodes = HashMap::new();
    nodes.insert(
        NodeId(0),
        ResolvedNode {
            id: NodeId(0),
            coordinate: "com.example:a".to_string(),
            requests: vec![VersionRequest {
                version: "1.0.0".to_string(),
                origin_module: "core".to_string(),
                depth: 1,
            }],
            selected: "1.0.0".to_string(),
            mediation_reason: MediationReason::SingleRequest,
        },
    );
    nodes.insert(
        NodeId(1),
        ResolvedNode {
            id: NodeId(1),
            coordinate: "com.example:b".to_string(),
            requests: vec![VersionRequest {
                version: "2.0.0".to_string(),
                origin_module: "core".to_string(),
                depth: 2,
            }],
            selected: "2.0.0".to_string(),
            mediation_reason: MediationReason::SingleRequest,
        },
    );
    let edges = vec![GraphEdge {
        from: NodeId(0),
        to: NodeId(1),
        kind: EdgeKind::External,
    }];
    DependencyGraph { nodes, edges }
}

#[test]
fn build_lockfile_from_mediated_graph() {
    let graph = sample_graph();
    let checksums = HashMap::from([
        ("com.example:a@1.0.0".to_string(), "sha-a".to_string()),
        ("com.example:b@2.0.0".to_string(), "sha-b".to_string()),
    ]);

    let lockfile = build_lockfile(
        &graph,
        "sha256:xyz".to_string(),
        &checksums,
        "default",
        "21",
    )
    .expect("should build");

    assert_eq!(lockfile.manifest_hash, "sha256:xyz");
    assert_eq!(lockfile.packages.len(), 2);

    let package_a = lockfile
        .packages
        .iter()
        .find(|p| p.name == "com.example:a")
        .unwrap();
    assert_eq!(package_a.version, "1.0.0");
    assert_eq!(package_a.sha256, "sha-a");
    assert_eq!(package_a.resolved_from, "default");
    assert_eq!(
        package_a.dependencies,
        vec!["com.example:b@2.0.0".to_string()]
    );

    let package_b = lockfile
        .packages
        .iter()
        .find(|p| p.name == "com.example:b")
        .unwrap();
    assert!(package_b.dependencies.is_empty());

    assert_eq!(lockfile.requests.len(), 2);
    assert!(lockfile
        .requests
        .iter()
        .any(|r| r.module == "core" && r.name == "com.example:a" && r.depth == 1));
}

#[test]
fn build_lockfile_missing_checksum_is_a_typed_error() {
    let graph = sample_graph();
    let checksums = HashMap::new();

    let result = build_lockfile(
        &graph,
        "sha256:xyz".to_string(),
        &checksums,
        "default",
        "21",
    );

    assert!(matches!(result, Err(LockfileError::MissingChecksum(_))));
}

#[test]
fn read_lockfile_returns_none_when_file_absent() {
    let path = temp_path("does-not-exist.lock");

    let result = read_lockfile(&path).expect("should not error");

    assert!(result.is_none());
}

#[test]
fn write_then_read_lockfile_round_trips() {
    let path = temp_path("roundtrip.lock");
    let lockfile = Lockfile {
        version: 1,
        manifest_hash: "sha256:roundtrip".to_string(),
        java_version: "21".to_string(),
        packages: vec![LockedPackage {
            name: "com.example:a".to_string(),
            version: "1.0.0".to_string(),
            sha256: "sha-a".to_string(),
            resolved_from: "default".to_string(),
            dependencies: Vec::new(),
        }],
        requests: Vec::new(),
    };

    write_lockfile(&path, &lockfile).expect("should write");
    let read_back = read_lockfile(&path)
        .expect("should read")
        .expect("should exist");

    assert_eq!(read_back, lockfile);

    let _ = std::fs::remove_file(&path);
}
