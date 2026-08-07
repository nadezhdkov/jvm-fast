use jvmfast::build::{build, module_order, BuildError};
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
            workspace_dependencies: Vec::new(),
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

fn module(name: &str, root: PathBuf, workspace_dependencies: Vec<String>) -> Module {
    Module {
        name: name.to_string(),
        root,
        declared_dependencies: Vec::new(),
        boms: Vec::new(),
        exclusions: Default::default(),
        workspace_dependencies,
    }
}

/// Real, end-to-end proof of the Fase 5 cross-module classpath: "api"
/// declares a `[workspace-dependencies]` on "core" and its source actually
/// imports a class `core` defines — this only compiles if `build::build`
/// (a) built `core` before `api` (topological order) and (b) put `core`'s
/// `target/classes` on `api`'s `javac -cp`.
#[test]
fn build_compiles_a_module_against_a_workspace_dependencys_classes() {
    let root = temp_dir("workspace-deps-e2e");
    let cache_root = temp_dir("workspace-deps-e2e-cache");
    let core_dir = root.join("core");
    let api_dir = root.join("api");

    std::fs::create_dir_all(core_dir.join("src/main/java/com/exemplo")).unwrap();
    std::fs::write(
        core_dir.join("src/main/java/com/exemplo/Greeter.java"),
        "package com.exemplo;\npublic class Greeter { public static String hello() { return \"hi\"; } }\n",
    )
    .unwrap();

    std::fs::create_dir_all(api_dir.join("src/main/java/com/exemplo")).unwrap();
    std::fs::write(
        api_dir.join("src/main/java/com/exemplo/Api.java"),
        "package com.exemplo;\npublic class Api { public static void main(String[] a) { Greeter.hello(); } }\n",
    )
    .unwrap();

    let workspace = Workspace {
        root: root.clone(),
        modules: vec![
            module("api", api_dir.clone(), vec!["core".to_string()]),
            module("core", core_dir.clone(), Vec::new()),
        ],
        lockfile: empty_lockfile(),
        config: WorkspaceConfig::default(),
    };

    let summaries = build(&workspace, &system_javac(), &cache_root).expect("should build");

    // Topological order: "core" (the dependency) must be built before
    // "api" (the dependent), even though `modules` above lists "api" first.
    assert_eq!(summaries[0].module, "core");
    assert_eq!(summaries[1].module, "api");
    assert!(api_dir
        .join("target/classes/com/exemplo/Api.class")
        .is_file());
    assert!(core_dir
        .join("target/classes/com/exemplo/Greeter.class")
        .is_file());

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_reports_an_unknown_workspace_dependency_as_a_typed_error() {
    let root = temp_dir("unknown-workspace-dep");
    let cache_root = temp_dir("unknown-workspace-dep-cache");
    let api_dir = root.join("api");
    std::fs::create_dir_all(&api_dir).unwrap();

    let workspace = Workspace {
        root: root.clone(),
        modules: vec![module("api", api_dir, vec!["nonexistent".to_string()])],
        lockfile: empty_lockfile(),
        config: WorkspaceConfig::default(),
    };

    let result = build(&workspace, &system_javac(), &cache_root);

    assert!(matches!(
        result,
        Err(BuildError::UnknownWorkspaceModule { module, dependency })
            if module == "api" && dependency == "nonexistent"
    ));

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn module_order_rejects_a_cycle_between_two_modules() {
    let modules = vec![
        module("a", PathBuf::from("a"), vec!["b".to_string()]),
        module("b", PathBuf::from("b"), vec!["a".to_string()]),
    ];

    let result = module_order(&modules);

    let Err(BuildError::CyclicModuleDependency(mut names)) = result else {
        panic!("expected a CyclicModuleDependency error");
    };
    names.sort();
    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn module_order_puts_dependencies_before_dependents() {
    let modules = vec![
        module("api", PathBuf::from("api"), vec!["core".to_string()]),
        module("core", PathBuf::from("core"), Vec::new()),
    ];

    let order = module_order(&modules).expect("should order without a cycle");

    assert_eq!(order, vec![1, 0]);
}

#[test]
fn build_skips_recompilation_when_nothing_changed() {
    let project_dir = temp_dir("incremental-no-change");
    let cache_root = temp_dir("incremental-no-change-cache");

    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/Main.java"),
        "public class Main {}\n",
    )
    .unwrap();

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());

    let first = build(&workspace, &system_javac(), &cache_root).expect("should build");
    assert!(!first[0].up_to_date);
    assert_eq!(first[0].compiled_files, 1);

    let second = build(&workspace, &system_javac(), &cache_root).expect("should build again");
    assert!(second[0].up_to_date);
    assert_eq!(second[0].compiled_files, 0);

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_recompiles_when_a_source_file_changes() {
    let project_dir = temp_dir("incremental-source-change");
    let cache_root = temp_dir("incremental-source-change-cache");

    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    let source_path = project_dir.join("src/main/java/Main.java");
    std::fs::write(&source_path, "public class Main {}\n").unwrap();

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());
    build(&workspace, &system_javac(), &cache_root).expect("should build");

    std::fs::write(&source_path, "public class Main { int x = 1; }\n").unwrap();
    let second = build(&workspace, &system_javac(), &cache_root).expect("should rebuild");

    assert!(!second[0].up_to_date);
    assert_eq!(second[0].compiled_files, 1);

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_recompiles_when_a_resource_file_changes() {
    let project_dir = temp_dir("incremental-resource-change");
    let cache_root = temp_dir("incremental-resource-change-cache");

    std::fs::create_dir_all(project_dir.join("src/main/resources")).unwrap();
    let resource_path = project_dir.join("src/main/resources/app.properties");
    std::fs::write(&resource_path, "key=1\n").unwrap();

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());
    build(&workspace, &system_javac(), &cache_root).expect("should build");

    std::fs::write(&resource_path, "key=2\n").unwrap();
    let second = build(&workspace, &system_javac(), &cache_root).expect("should rebuild");

    assert!(!second[0].up_to_date);
    assert_eq!(
        std::fs::read_to_string(project_dir.join("target/classes/app.properties")).unwrap(),
        "key=2\n"
    );

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

#[test]
fn build_treats_a_missing_target_classes_as_never_built() {
    let project_dir = temp_dir("incremental-missing-classes");
    let cache_root = temp_dir("incremental-missing-classes-cache");

    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/Main.java"),
        "public class Main {}\n",
    )
    .unwrap();

    let workspace = workspace_with_module(project_dir.clone(), empty_lockfile());
    build(&workspace, &system_javac(), &cache_root).expect("should build");

    std::fs::remove_dir_all(project_dir.join("target/classes")).unwrap();
    let second = build(&workspace, &system_javac(), &cache_root).expect("should rebuild");

    assert!(!second[0].up_to_date);
    assert!(project_dir.join("target/classes/Main.class").is_file());

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&cache_root);
}

/// The transitive-invalidation case: "api" doesn't touch its own files at
/// all between builds, but "core" (its workspace dependency) does — "api"
/// must still be recompiled, since a stale reference to `core`'s previous
/// class shape would be wrong. This is exactly why `module.workspace_dependencies`'
/// *fingerprints* (not just their file contents) feed into a module's own
/// fingerprint (`fingerprint::compute_module_fingerprint`).
#[test]
fn build_transitively_invalidates_a_dependent_when_its_workspace_dependency_changes() {
    let root = temp_dir("incremental-transitive");
    let cache_root = temp_dir("incremental-transitive-cache");
    let core_dir = root.join("core");
    let api_dir = root.join("api");

    std::fs::create_dir_all(core_dir.join("src/main/java")).unwrap();
    let core_source = core_dir.join("src/main/java/Greeter.java");
    std::fs::write(
        &core_source,
        "public class Greeter { public static String hello() { return \"hi\"; } }\n",
    )
    .unwrap();

    std::fs::create_dir_all(api_dir.join("src/main/java")).unwrap();
    std::fs::write(
        api_dir.join("src/main/java/Api.java"),
        "public class Api { public static void main(String[] a) { Greeter.hello(); } }\n",
    )
    .unwrap();

    let workspace = Workspace {
        root: root.clone(),
        modules: vec![
            module("api", api_dir.clone(), vec!["core".to_string()]),
            module("core", core_dir.clone(), Vec::new()),
        ],
        lockfile: empty_lockfile(),
        config: WorkspaceConfig::default(),
    };

    let first = build(&workspace, &system_javac(), &cache_root).expect("should build");
    assert!(!first[0].up_to_date && !first[1].up_to_date);

    // Nothing under api/ changes — only core's own source.
    std::fs::write(
        &core_source,
        "public class Greeter { public static String hello() { return \"hi there\"; } }\n",
    )
    .unwrap();

    let second = build(&workspace, &system_javac(), &cache_root).expect("should rebuild");
    let core_summary = second.iter().find(|s| s.module == "core").unwrap();
    let api_summary = second.iter().find(|s| s.module == "api").unwrap();
    assert!(!core_summary.up_to_date, "core itself changed");
    assert!(
        !api_summary.up_to_date,
        "api must be invalidated transitively even though its own files didn't change"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache_root);
}

/// Mirror case: if *neither* module's inputs changed, both stay up to date
/// on a second build — proves the transitive-invalidation test above isn't
/// just "always rebuilds everything" in disguise.
#[test]
fn build_keeps_both_modules_up_to_date_when_nothing_changed() {
    let root = temp_dir("incremental-transitive-stable");
    let cache_root = temp_dir("incremental-transitive-stable-cache");
    let core_dir = root.join("core");
    let api_dir = root.join("api");

    std::fs::create_dir_all(core_dir.join("src/main/java")).unwrap();
    std::fs::write(
        core_dir.join("src/main/java/Greeter.java"),
        "public class Greeter { public static String hello() { return \"hi\"; } }\n",
    )
    .unwrap();
    std::fs::create_dir_all(api_dir.join("src/main/java")).unwrap();
    std::fs::write(
        api_dir.join("src/main/java/Api.java"),
        "public class Api { public static void main(String[] a) { Greeter.hello(); } }\n",
    )
    .unwrap();

    let workspace = Workspace {
        root: root.clone(),
        modules: vec![
            module("api", api_dir, vec!["core".to_string()]),
            module("core", core_dir, Vec::new()),
        ],
        lockfile: empty_lockfile(),
        config: WorkspaceConfig::default(),
    };

    build(&workspace, &system_javac(), &cache_root).expect("should build");
    let second = build(&workspace, &system_javac(), &cache_root).expect("should build again");

    assert!(second.iter().all(|s| s.up_to_date));

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&cache_root);
}
