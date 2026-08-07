mod support;

use jvmfast::cache::hash_bytes;
use jvmfast::cli::{build, install};
use std::path::PathBuf;
use support::start_mock_server;
use tokio::sync::Mutex;

/// Same discipline as `tests/cli_install.rs`: `install`/`build` resolve
/// `cache::CacheStore`'s root and the JDKs root from `$HOME` (seção 5),
/// which is process-global state — serialize every test in this file that
/// touches it.
static HOME_GUARD: Mutex<()> = Mutex::const_new(());

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-multi-module-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

const EMPTY_POM: &str = r#"<project>
  <dependencies>
  </dependencies>
</project>"#;

/// Exercises the Fase 5 foundation end to end: a root module plus a
/// `[workspace].members` module, each declaring a distinct dependency,
/// resolved and downloaded together by `jvmfast install`, then compiled
/// (as two independent `target/classes` trees) by `jvmfast build` — the
/// same two commands that already worked for a single module, now proven
/// to actually operate on N real modules loaded from disk rather than a
/// hardcoded `vec![module]`.
#[tokio::test]
async fn install_and_build_operate_on_every_workspace_module() {
    let _guard = HOME_GUARD.lock().await;

    let root_jar = b"root module's own dependency".to_vec();
    let root_sha256 = hash_bytes(&root_jar);
    let core_jar = b"core module's own dependency".to_vec();
    let core_sha256 = hash_bytes(&core_jar);

    let root_jar_clone = root_jar.clone();
    let root_sha256_clone = root_sha256.clone();
    let core_jar_clone = core_jar.clone();
    let core_sha256_clone = core_sha256.clone();
    let server = start_mock_server(move |path| match path {
        "/com/example/root-lib/1.0.0/root-lib-1.0.0.pom" => (200, EMPTY_POM.as_bytes().to_vec()),
        "/com/example/root-lib/1.0.0/root-lib-1.0.0.jar" => (200, root_jar_clone.clone()),
        "/com/example/root-lib/1.0.0/root-lib-1.0.0.jar.sha256" => {
            (200, root_sha256_clone.clone().into_bytes())
        }
        "/com/example/core-lib/2.0.0/core-lib-2.0.0.pom" => (200, EMPTY_POM.as_bytes().to_vec()),
        "/com/example/core-lib/2.0.0/core-lib-2.0.0.jar" => (200, core_jar_clone.clone()),
        "/com/example/core-lib/2.0.0/core-lib-2.0.0.jar.sha256" => {
            (200, core_sha256_clone.clone().into_bytes())
        }
        _ => (404, Vec::new()),
    });

    let project_dir = temp_dir("project");
    let home_dir = temp_dir("home");
    std::fs::create_dir_all(home_dir.join(".cache/jvmfast/jdks/21.0.1-tem")).unwrap();

    let root_manifest = format!(
        "[project]\nname = \"root-module\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
         [workspace]\nmembers = [\"core\"]\n\n\
         [dependencies]\n\"com.example:root-lib\" = \"1.0.0\"\n\n\
         [repositories]\ndefault = \"{}\"\n",
        server.base_url
    );
    std::fs::write(project_dir.join("project.toml"), root_manifest).unwrap();

    std::fs::create_dir_all(project_dir.join("core")).unwrap();
    let core_manifest =
        "[project]\nname = \"core\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
         [dependencies]\n\"com.example:core-lib\" = \"2.0.0\"\n";
    std::fs::write(project_dir.join("core/project.toml"), core_manifest).unwrap();

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let install_result = install(&project_dir, false, false).await;

    let build_result = build(&project_dir);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    let summary = install_result.expect("install should succeed across both modules");
    assert_eq!(summary.package_count, 2);
    assert_eq!(summary.downloaded_count, 2);

    let lockfile_contents =
        std::fs::read_to_string(project_dir.join("project.lock")).expect("lock should exist");
    assert!(lockfile_contents.contains("com.example:root-lib"));
    assert!(lockfile_contents.contains("com.example:core-lib"));
    // Each [[request]] carries which module actually declared it (seção
    // 6.2/13.1) — this is the domain-level provenance that already worked
    // (tests/graph_construction.rs), now proven wired all the way through
    // real manifest loading and the lockfile written to disk.
    assert!(lockfile_contents.contains("module = \"root-module\""));
    assert!(lockfile_contents.contains("module = \"core\""));

    let build_summary = build_result.expect("build should succeed across both modules");
    assert!(build_summary.contains("2 module(s) rebuilt, 0 up to date"));
    assert!(project_dir.join("target/classes").is_dir());
    assert!(project_dir.join("core/target/classes").is_dir());

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}
