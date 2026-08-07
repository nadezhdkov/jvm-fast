mod support;

use jvmfast::cache::hash_bytes;
use jvmfast::cli::install;
use std::path::PathBuf;
use support::start_mock_server;
use tokio::sync::Mutex;

/// `install`/`update` resolvem `cache::CacheStore`'s root a partir de
/// `$HOME` (seção 5) — mutar essa env var globalmente exige serializar
/// qualquer teste deste arquivo que dependa dela, já que `cargo test`
/// roda testes do mesmo binário em threads concorrentes por padrão.
/// `tokio::sync::Mutex` (não `std::sync::Mutex`) porque o guard precisa
/// atravessar pontos de `.await`.
static HOME_GUARD: Mutex<()> = Mutex::const_new(());

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-install-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

const DEMO_POM: &str = r#"<project>
  <dependencies>
  </dependencies>
</project>"#;

#[tokio::test]
async fn install_resolves_downloads_and_writes_lockfile_end_to_end() {
    let _guard = HOME_GUARD.lock().await;

    let jar_contents = b"pretend jar bytes for com.example:demo:1.0.0".to_vec();
    let sha256 = hash_bytes(&jar_contents);
    let jar_contents_clone = jar_contents.clone();
    let sha256_clone = sha256.clone();

    let server = start_mock_server(move |path| match path {
        "/com/example/demo/1.0.0/demo-1.0.0.pom" => (200, DEMO_POM.as_bytes().to_vec()),
        "/com/example/demo/1.0.0/demo-1.0.0.jar" => (200, jar_contents_clone.clone()),
        "/com/example/demo/1.0.0/demo-1.0.0.jar.sha256" => (200, sha256_clone.clone().into_bytes()),
        _ => (404, Vec::new()),
    });

    let project_dir = temp_dir("project");
    let home_dir = temp_dir("home");
    // Pré-instala uma JDK 21 "fake" (só o diretório precisa existir —
    // `jdk::list_installed` não valida conteúdo) para que a resolução de
    // `[project].java-version = "21"` embutida em `install` não tente
    // baixar nada da Adoptium real nem pedir confirmação interativa.
    std::fs::create_dir_all(home_dir.join(".cache/jvmfast/jdks/21.0.1-tem")).unwrap();
    let manifest = format!(
        "[project]\nname = \"demo-project\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
         [dependencies]\n\"com.example:demo\" = \"1.0.0\"\n\n\
         [repositories]\ndefault = \"{}\"\n",
        server.base_url
    );
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let result = install(&project_dir, false, false).await;

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    let summary = result.expect("install should succeed");
    assert_eq!(summary.package_count, 1);
    assert_eq!(summary.downloaded_count, 1);
    assert_eq!(summary.reused_from_cache_count, 0);
    assert!(!summary.reused_lockfile);

    let lockfile_contents =
        std::fs::read_to_string(project_dir.join("project.lock")).expect("lock should exist");
    assert!(lockfile_contents.contains("com.example:demo"));
    assert!(lockfile_contents.contains(&sha256));
    assert!(lockfile_contents.contains("java-version = \"21\""));

    let cached_jar = home_dir
        .join(".cache/jvmfast/artifacts/sha256")
        .join(&sha256[0..2])
        .join(&sha256[2..4])
        .join(&sha256)
        .join("demo-1.0.0.jar");
    assert!(cached_jar.is_file());
    assert_eq!(std::fs::read(&cached_jar).unwrap(), jar_contents);

    // Uma segunda `install`, agora com o lock já válido, deve pular a
    // resolução inteira (seção 6.2 passo 2) e só constatar que o artefato
    // já está em cache.
    std::env::set_var("HOME", &home_dir);
    let second = install(&project_dir, false, false).await;
    std::env::remove_var("HOME");
    let second_summary = second.expect("second install should succeed");
    assert!(second_summary.reused_lockfile);
    assert_eq!(second_summary.downloaded_count, 0);
    assert_eq!(second_summary.reused_from_cache_count, 1);

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}
