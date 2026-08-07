use jvmfast::cli::{build, CliError};
use jvmfast::domain::Lockfile;
use jvmfast::lockfile::{compute_manifest_hash, write_lockfile};
use std::path::PathBuf;
use tokio::sync::Mutex;

/// `cli::build` resolve `cache_root`/`jdks_root` a partir de `$HOME`
/// (mesmo padrão de `tests/cli_install.rs`/`tests/cli_jdk.rs`) — mutar essa
/// env var globalmente exige serializar os testes deste arquivo.
static HOME_GUARD: Mutex<()> = Mutex::const_new(());

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-build-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

const MANIFEST: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";

/// Instala uma JDK "fake" cujo `bin/javac` é na verdade um symlink pro
/// `javac` real do ambiente de teste (seção 7 do plano assume Unix, mesma
/// disciplina de `cache::cache_root`) — evita depender de `jdk::install`
/// (rede/mocks) só para exercitar `cli::build`, que só precisa de um path
/// de `javac` que funcione de verdade.
fn install_fake_jdk(jdks_root: &std::path::Path, dir_name: &str) {
    let bin_dir = jdks_root.join(dir_name).join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let real_javac = PathBuf::from("/usr/bin/javac");
    std::os::unix::fs::symlink(&real_javac, bin_dir.join("javac")).unwrap();
}

#[tokio::test]
async fn build_compiles_project_using_the_locked_java_version() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("project");
    let home_dir = temp_dir("home");

    std::fs::write(project_dir.join("project.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/Main.java"),
        "public class Main { public static void main(String[] a) {} }\n",
    )
    .unwrap();

    let manifest_hash = compute_manifest_hash([MANIFEST]);
    let lockfile = Lockfile {
        version: 1,
        manifest_hash,
        java_version: "21".to_string(),
        packages: Vec::new(),
        requests: Vec::new(),
    };
    write_lockfile(&project_dir.join("project.lock"), &lockfile).unwrap();

    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let result = build(&project_dir);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    let message = result.expect("build should succeed");
    assert!(message.contains("compiled 1 source file"));
    assert!(project_dir.join("target/classes/Main.class").is_file());

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn build_rejects_missing_lockfile() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("no-lock");
    std::fs::write(project_dir.join("project.toml"), MANIFEST).unwrap();

    let result = build(&project_dir);

    assert!(matches!(result, Err(CliError::LockfileMissing)));

    let _ = std::fs::remove_dir_all(&project_dir);
}

#[tokio::test]
async fn build_rejects_stale_lockfile() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("stale-lock");
    std::fs::write(project_dir.join("project.toml"), MANIFEST).unwrap();

    let lockfile = Lockfile {
        version: 1,
        manifest_hash: "sha256:stale".to_string(),
        java_version: "21".to_string(),
        packages: Vec::new(),
        requests: Vec::new(),
    };
    write_lockfile(&project_dir.join("project.lock"), &lockfile).unwrap();

    let result = build(&project_dir);

    assert!(matches!(result, Err(CliError::LockfileStale)));

    let _ = std::fs::remove_dir_all(&project_dir);
}

#[tokio::test]
async fn build_rejects_when_locked_java_version_is_not_installed() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("jdk-missing");
    let home_dir = temp_dir("jdk-missing-home");
    std::fs::write(project_dir.join("project.toml"), MANIFEST).unwrap();

    let manifest_hash = compute_manifest_hash([MANIFEST]);
    let lockfile = Lockfile {
        version: 1,
        manifest_hash,
        java_version: "21".to_string(),
        packages: Vec::new(),
        requests: Vec::new(),
    };
    write_lockfile(&project_dir.join("project.lock"), &lockfile).unwrap();

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let result = build(&project_dir);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(matches!(
        result,
        Err(CliError::JavaVersionNotInstalled(v)) if v == "21"
    ));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}
