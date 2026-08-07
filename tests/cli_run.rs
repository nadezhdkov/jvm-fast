use jvmfast::cli::{run_program, CliError};
use jvmfast::domain::Lockfile;
use jvmfast::lockfile::{compute_manifest_hash, write_lockfile};
use std::path::PathBuf;
use tokio::sync::Mutex;

/// Mesmo padrão de `tests/cli_build.rs` — `cli::run_program` resolve
/// `cache_root`/`jdks_root` a partir de `$HOME`, então testes deste arquivo
/// que mutam essa env var precisam ser serializados.
static HOME_GUARD: Mutex<()> = Mutex::const_new(());

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-run-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn install_fake_jdk(jdks_root: &std::path::Path, dir_name: &str) {
    let bin_dir = jdks_root.join(dir_name).join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::os::unix::fs::symlink("/usr/bin/javac", bin_dir.join("javac")).unwrap();
    std::os::unix::fs::symlink("/usr/bin/java", bin_dir.join("java")).unwrap();
}

fn write_valid_lockfile(project_dir: &std::path::Path, manifest: &str) {
    let manifest_hash = compute_manifest_hash([manifest]);
    let lockfile = Lockfile {
        version: 1,
        manifest_hash,
        java_version: "21".to_string(),
        packages: Vec::new(),
        requests: Vec::new(),
    };
    write_lockfile(&project_dir.join("project.lock"), &lockfile).unwrap();
}

#[tokio::test]
async fn run_compiles_and_executes_the_configured_main_class() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("project");
    let home_dir = temp_dir("home");

    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                     [run]\nmain-class = \"Main\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/Main.java"),
        "public class Main { public static void main(String[] a) { \
         System.out.println(\"hello from run\"); } }\n",
    )
    .unwrap();
    write_valid_lockfile(&project_dir, manifest);
    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);
    let result = run_program(&project_dir, None);
    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    let message = result.expect("run should succeed");
    assert!(message.contains("Main"));
    assert!(project_dir.join("target/classes/Main.class").is_file());

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn run_propagates_non_zero_exit_as_typed_error() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("nonzero");
    let home_dir = temp_dir("nonzero-home");

    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                     [run]\nmain-class = \"Failing\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/Failing.java"),
        "public class Failing { public static void main(String[] a) { System.exit(3); } }\n",
    )
    .unwrap();
    write_valid_lockfile(&project_dir, manifest);
    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);
    let result = run_program(&project_dir, None);
    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(matches!(result, Err(CliError::ProgramExited(3))));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn run_rejects_missing_main_class_configuration() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("no-main-class");
    let home_dir = temp_dir("no-main-class-home");

    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();
    write_valid_lockfile(&project_dir, manifest);
    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);
    let result = run_program(&project_dir, None);
    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(matches!(result, Err(CliError::MainClassNotConfigured)));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

/// Fase 5: `--module` picks which module's `[run]` gets executed — "root"
/// declares one main class, "worker" (a `[workspace].members` entry)
/// declares a different one, and each must run its *own*.
#[tokio::test]
async fn run_selects_the_configured_module_via_module_flag() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("module-flag");
    let home_dir = temp_dir("module-flag-home");
    let worker_dir = project_dir.join("worker");

    let root_manifest =
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                          [workspace]\nmembers = [\"worker\"]\n\n\
                          [run]\nmain-class = \"RootMain\"\n";
    std::fs::write(project_dir.join("project.toml"), root_manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/main/java")).unwrap();
    std::fs::write(
        project_dir.join("src/main/java/RootMain.java"),
        "public class RootMain { public static void main(String[] a) { \
         System.out.println(\"hello from root\"); } }\n",
    )
    .unwrap();

    let worker_manifest =
        "[project]\nname = \"worker\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                            [run]\nmain-class = \"WorkerMain\"\n";
    std::fs::create_dir_all(worker_dir.join("src/main/java")).unwrap();
    std::fs::write(worker_dir.join("project.toml"), worker_manifest).unwrap();
    std::fs::write(
        worker_dir.join("src/main/java/WorkerMain.java"),
        "public class WorkerMain { public static void main(String[] a) { \
         System.out.println(\"hello from worker\"); } }\n",
    )
    .unwrap();

    let manifest_hash = compute_manifest_hash([root_manifest, worker_manifest]);
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
    let root_result = run_program(&project_dir, None);
    let worker_result = run_program(&project_dir, Some("worker".to_string()));
    let unknown_result = run_program(&project_dir, Some("nonexistent".to_string()));
    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(root_result
        .expect("root run should succeed")
        .contains("RootMain"));
    assert!(worker_result
        .expect("worker run should succeed")
        .contains("WorkerMain"));
    assert!(matches!(
        unknown_result,
        Err(CliError::ModuleNotFound(name)) if name == "nonexistent"
    ));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn run_rejects_missing_lockfile() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("no-lock");
    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();

    let result = run_program(&project_dir, None);

    assert!(matches!(result, Err(CliError::LockfileMissing)));

    let _ = std::fs::remove_dir_all(&project_dir);
}
