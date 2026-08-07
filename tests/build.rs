use jvmfast::build::{build, BuildError};
use jvmfast::cache::CacheStore;
use jvmfast::domain::{LockedPackage, Lockfile, Module, Workspace, WorkspaceConfig};
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-build-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn system_javac() -> PathBuf {
    // Ambiente de teste tem uma JDK real instalada (`javac`/`java` no
    // PATH) — usada diretamente aqui em vez de uma JDK baixada via
    // `jdk::install` (marco anterior), já que `build::compile` só precisa
    // de um path pra `javac`, não de nada específico do Adoptium.
    PathBuf::from("javac")
}

fn empty_lockfile() -> Lockfile {
    Lockfile {
        version: 1,
        manifest_hash: "sha256:test".to_string(),
        java_version: "21".to_string(),
        packages: Vec::new(),
        requests: Vec::new(),
    }
}

fn workspace_with_module(root: PathBuf, lockfile: Lockfile) -> Workspace {
    Workspace {
        root: root.clone(),
        modules: vec![Module {
            name: "demo".to_string(),
            root,
            declared_dependencies: Vec::new(),
            boms: Vec::new(),
            exclusions: Default::default(),
        }],
        lockfile,
        config: WorkspaceConfig::default(),
    }
}

#[test]
fn build_compiles_sources_and_copies_resources_end_to_end() {
    let project_dir = temp_dir("e2e");
    let cache_root = temp_dir("e2e-cache");

    std::fs::create_dir_all(project_dir.join("src/main/java/com/exemplo")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/com/exemplo/Main.java"),
        "package com.exemplo;\npublic class Main { public static void main(String[] a) {} }\n",
    )
    .unwrap();

    std::fs::create_dir_all(project_dir.join("src/main/resources")).unwrap();
    std::fs::write(
        project_dir.join("src/main/resources/application.properties"),
        "key=value\n",
    )
    .unwrap();

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());
    let summaries = build(&workspace, &system_javac(), &cache_root).expect("should build");

    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert_eq!(summary.module, "demo");
    assert_eq!(summary.compiled_files, 1);
    assert_eq!(summary.copied_resources, 1);

    let class_file = project_dir.join("target/classes/com/exemplo/Main.class");
    assert!(class_file.is_file());
    let copied_resource = project_dir.join("target/classes/application.properties");
    assert!(copied_resource.is_file());
    assert_eq!(
        std::fs::read_to_string(&copied_resource).unwrap(),
        "key=value\n"
    );

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_with_no_sources_still_creates_target_classes() {
    let project_dir = temp_dir("no-sources");
    let cache_root = temp_dir("no-sources-cache");

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());
    let summaries = build(&workspace, &system_javac(), &cache_root).expect("should build");

    assert_eq!(summaries[0].compiled_files, 0);
    assert!(project_dir.join("target/classes").is_dir());

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_reports_typed_compile_error_on_invalid_source() {
    let project_dir = temp_dir("compile-error");
    let cache_root = temp_dir("compile-error-cache");

    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/Broken.java"),
        "this is not valid java",
    )
    .unwrap();

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());
    let result = build(&workspace, &system_javac(), &cache_root);

    assert!(matches!(result, Err(BuildError::CompileFailed { .. })));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_reports_typed_error_when_locked_artifact_is_not_cached() {
    let project_dir = temp_dir("missing-artifact");
    let cache_root = temp_dir("missing-artifact-cache");

    let mut lockfile = empty_lockfile();
    lockfile.packages.push(LockedPackage {
        name: "com.example:demo".to_string(),
        version: "1.0.0".to_string(),
        sha256: "0".repeat(64),
        resolved_from: "default".to_string(),
        dependencies: Vec::new(),
    });

    let workspace = workspace_with_module(project_dir.clone(), lockfile);
    let result = build(&workspace, &system_javac(), &cache_root);

    assert!(
        matches!(result, Err(BuildError::MissingArtifact(coord)) if coord == "com.example:demo@1.0.0")
    );

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_uses_cached_artifact_on_classpath_when_present() {
    let project_dir = temp_dir("with-dependency");
    let cache_root = temp_dir("with-dependency-cache");

    let jar_bytes = b"pretend jar bytes";
    let sha256 = jvmfast::cache::hash_bytes(jar_bytes);
    let cache_store = CacheStore::new(&cache_root);
    cache_store
        .write_artifact(jar_bytes, &sha256, "demo-1.0.0.jar")
        .expect("should seed cache");

    let mut lockfile = empty_lockfile();
    lockfile.packages.push(LockedPackage {
        name: "com.example:demo".to_string(),
        version: "1.0.0".to_string(),
        sha256,
        resolved_from: "default".to_string(),
        dependencies: Vec::new(),
    });

    let workspace = workspace_with_module(project_dir.clone(), lockfile);
    let summaries = build(&workspace, &system_javac(), &cache_root).expect("should build");

    assert_eq!(summaries[0].compiled_files, 0);

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}
