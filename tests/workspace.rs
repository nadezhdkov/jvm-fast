use jvmfast::lockfile::compute_manifest_hash;
use jvmfast::workspace::{current_manifest_hash, load_workspace, WorkspaceLoadError};
use std::path::PathBuf;

fn temp_project_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-workspace-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

const MINIMAL_MANIFEST: &str =
    "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";

#[test]
fn load_workspace_creates_empty_lockfile_when_absent() {
    let dir = temp_project_dir("no-lock");
    std::fs::write(dir.join("project.toml"), MINIMAL_MANIFEST).unwrap();

    let workspace = load_workspace(&dir).expect("should load");

    assert_eq!(workspace.modules.len(), 1);
    assert_eq!(workspace.modules[0].name, "demo");
    assert!(workspace.lockfile.packages.is_empty());
    assert!(workspace.lockfile.requests.is_empty());
    assert_eq!(
        workspace.lockfile.manifest_hash,
        compute_manifest_hash([MINIMAL_MANIFEST])
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_workspace_loads_existing_lockfile_from_disk() {
    let dir = temp_project_dir("with-lock");
    std::fs::write(dir.join("project.toml"), MINIMAL_MANIFEST).unwrap();
    std::fs::write(
        dir.join("project.lock"),
        "version = 1\nmanifest-hash = \"sha256:stale\"\njava-version = \"21\"\n",
    )
    .unwrap();

    let workspace = load_workspace(&dir).expect("should load");

    // O lock em disco é carregado como está — mesmo com hash desatualizado;
    // decidir se está válido é responsabilidade de `is_lockfile_valid`, não
    // de `load_workspace`.
    assert_eq!(workspace.lockfile.manifest_hash, "sha256:stale");
    assert_ne!(
        current_manifest_hash(&dir).expect("should compute"),
        workspace.lockfile.manifest_hash
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn current_manifest_hash_matches_a_freshly_loaded_workspace_without_lockfile() {
    let dir = temp_project_dir("current-hash");
    std::fs::write(dir.join("project.toml"), MINIMAL_MANIFEST).unwrap();

    let workspace = load_workspace(&dir).expect("should load");
    let hash = current_manifest_hash(&dir).expect("should compute");

    assert_eq!(hash, workspace.lockfile.manifest_hash);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_workspace_uses_documented_config_defaults() {
    let dir = temp_project_dir("config-defaults");
    std::fs::write(dir.join("project.toml"), MINIMAL_MANIFEST).unwrap();

    let workspace = load_workspace(&dir).expect("should load");

    assert_eq!(workspace.config.network.connect_timeout_secs, 10);
    assert_eq!(workspace.config.network.max_retries, 3);
    assert!(workspace.config.output.progress_bar);

    let _ = std::fs::remove_dir_all(&dir);
}

const ROOT_MANIFEST_WITH_MEMBERS: &str = "[project]\nname = \"root\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n[workspace]\nmembers = [\"core\", \"api\"]\n";
const CORE_MANIFEST: &str =
    "[project]\nname = \"core\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";
const API_MANIFEST: &str =
    "[project]\nname = \"api\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";

#[test]
fn load_workspace_loads_root_plus_declared_members() {
    let dir = temp_project_dir("multi-module");
    std::fs::write(dir.join("project.toml"), ROOT_MANIFEST_WITH_MEMBERS).unwrap();
    std::fs::create_dir_all(dir.join("core")).unwrap();
    std::fs::write(dir.join("core/project.toml"), CORE_MANIFEST).unwrap();
    std::fs::create_dir_all(dir.join("api")).unwrap();
    std::fs::write(dir.join("api/project.toml"), API_MANIFEST).unwrap();

    let workspace = load_workspace(&dir).expect("should load");

    let names: Vec<&str> = workspace
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    assert_eq!(names, vec!["root", "core", "api"]);
    assert_eq!(workspace.modules[1].root, dir.join("core"));
    assert_eq!(workspace.modules[2].root, dir.join("api"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_workspace_hashes_root_and_member_manifests_together_in_order() {
    let dir = temp_project_dir("multi-module-hash");
    std::fs::write(dir.join("project.toml"), ROOT_MANIFEST_WITH_MEMBERS).unwrap();
    std::fs::create_dir_all(dir.join("core")).unwrap();
    std::fs::write(dir.join("core/project.toml"), CORE_MANIFEST).unwrap();
    std::fs::create_dir_all(dir.join("api")).unwrap();
    std::fs::write(dir.join("api/project.toml"), API_MANIFEST).unwrap();

    let workspace = load_workspace(&dir).expect("should load");
    let hash = current_manifest_hash(&dir).expect("should compute");

    assert_eq!(hash, workspace.lockfile.manifest_hash);
    assert_eq!(
        hash,
        compute_manifest_hash([ROOT_MANIFEST_WITH_MEMBERS, CORE_MANIFEST, API_MANIFEST])
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_workspace_without_a_workspace_table_still_loads_a_single_module() {
    // Backward-compat check: a manifest with no `[workspace]` table at all
    // (the common, pre-Fase-5 case) must keep behaving exactly as before.
    let dir = temp_project_dir("no-workspace-table");
    std::fs::write(dir.join("project.toml"), MINIMAL_MANIFEST).unwrap();

    let workspace = load_workspace(&dir).expect("should load");

    assert_eq!(workspace.modules.len(), 1);
    assert_eq!(workspace.modules[0].name, "demo");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_workspace_rejects_a_member_with_a_duplicate_module_name() {
    let dir = temp_project_dir("duplicate-name");
    let root_manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n[workspace]\nmembers = [\"other\"]\n";
    std::fs::write(dir.join("project.toml"), root_manifest).unwrap();
    std::fs::create_dir_all(dir.join("other")).unwrap();
    // Same module name ("demo") as the root — must be rejected, not silently
    // merged/shadowed, since diagnostics (VersionRequest.origin_module,
    // LockedRequest.module) are keyed by module name.
    std::fs::write(dir.join("other/project.toml"), MINIMAL_MANIFEST).unwrap();

    let result = load_workspace(&dir);

    assert!(matches!(
        result,
        Err(WorkspaceLoadError::DuplicateModuleName(name)) if name == "demo"
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_workspace_reports_a_missing_member_manifest_as_a_typed_io_error() {
    let dir = temp_project_dir("missing-member");
    std::fs::write(dir.join("project.toml"), ROOT_MANIFEST_WITH_MEMBERS).unwrap();
    // Neither `core/` nor `api/` exists on disk.

    let result = load_workspace(&dir);

    assert!(matches!(result, Err(WorkspaceLoadError::Io { .. })));

    let _ = std::fs::remove_dir_all(&dir);
}
