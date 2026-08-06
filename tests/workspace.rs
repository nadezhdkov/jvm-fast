use jvmfast::lockfile::compute_manifest_hash;
use jvmfast::workspace::load_workspace;
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
        "version = 1\nmanifest-hash = \"sha256:stale\"\n",
    )
    .unwrap();

    let workspace = load_workspace(&dir).expect("should load");

    // O lock em disco é carregado como está — mesmo com hash desatualizado;
    // decidir se está válido é responsabilidade de `is_lockfile_valid`, não
    // de `load_workspace`.
    assert_eq!(workspace.lockfile.manifest_hash, "sha256:stale");

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
