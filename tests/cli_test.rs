mod support;

use jvmfast::cache::hash_bytes;
use jvmfast::cli::{install, test, CliError, TestOptions};
use std::path::PathBuf;
use tokio::sync::Mutex;

/// Mesmo padrão de `tests/cli_build.rs`/`tests/cli_run.rs` — `cli::test`
/// resolve `cache_root`/`jdks_root` a partir de `$HOME`.
static HOME_GUARD: Mutex<()> = Mutex::const_new(());

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-test-{}-{}",
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

/// Compila uma classe Java de pacote-default e a empacota num `.jar` real
/// via `javac`/`jar` do sistema — usado pra montar um artefato "de
/// dependência" de verdade (bytes reais, carregável em runtime) que o
/// mock HTTP server abaixo serve, sem precisar de rede real nem de um jar
/// pré-fabricado versionado no repo.
fn build_fake_dependency_jar(class_name: &str, source: &str) -> Vec<u8> {
    let build_dir = temp_dir(&format!("depjar-{class_name}"));
    std::fs::write(build_dir.join(format!("{class_name}.java")), source).unwrap();

    let javac_status = std::process::Command::new("javac")
        .arg("-d")
        .arg(&build_dir)
        .arg(build_dir.join(format!("{class_name}.java")))
        .status()
        .expect("should run javac");
    assert!(javac_status.success());

    let jar_path = build_dir.join("out.jar");
    let jar_status = std::process::Command::new("jar")
        .arg("cf")
        .arg(&jar_path)
        .arg("-C")
        .arg(&build_dir)
        .arg(format!("{class_name}.class"))
        .status()
        .expect("should run jar");
    assert!(jar_status.success());

    let bytes = std::fs::read(&jar_path).unwrap();
    let _ = std::fs::remove_dir_all(&build_dir);
    bytes
}

fn no_options() -> TestOptions {
    TestOptions {
        filter: None,
        fail_fast: false,
        report_xml: false,
    }
}

/// `console::ensure_console_jar` sempre busca o JUnit Platform Console
/// Standalone do Maven Central real (deliberado — é uma ferramenta interna
/// do jvm-fast, não uma dependência do projeto sob teste, ver
/// `src/testing/console.rs`), então este teste é, junto com
/// `tests/build.rs`/`tests/run.rs`, uma das únicas exceções deste repo à
/// regra de nunca tocar rede real em teste (docs/CONVENTIONS.md) — o
/// próprio repositório do *projeto* (`[repositories].default`) continua
/// sendo um mock local, só o download do Console Launcher em si é real.
#[tokio::test]
async fn test_compiles_and_runs_a_passing_test_class_against_real_junit_console() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("passing");
    let home_dir = temp_dir("passing-home");

    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                     [repositories]\ndefault = \"https://repo1.maven.org/maven2\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/test/java")).unwrap();
    std::fs::write(
        project_dir.join("src/test/java/PassingTest.java"),
        "import org.junit.jupiter.api.Test;\n\
         import static org.junit.jupiter.api.Assertions.assertEquals;\n\
         class PassingTest {\n    @Test void addsUp() { assertEquals(4, 2 + 2); }\n}\n",
    )
    .unwrap();

    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let install_result = install(&project_dir, false, true).await;
    let test_result = test(&project_dir, no_options()).await;

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    install_result.expect("install should succeed");
    let message = test_result.expect("test should succeed");
    assert!(message.contains("all tests passed"));
    assert!(project_dir
        .join("target/test-classes/PassingTest.class")
        .is_file());

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn test_reports_typed_error_when_a_test_fails() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("failing");
    let home_dir = temp_dir("failing-home");

    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                     [repositories]\ndefault = \"https://repo1.maven.org/maven2\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/test/java")).unwrap();
    std::fs::write(
        project_dir.join("src/test/java/FailingTest.java"),
        "import org.junit.jupiter.api.Test;\n\
         import static org.junit.jupiter.api.Assertions.assertEquals;\n\
         class FailingTest {\n    @Test void broken() { assertEquals(5, 2 + 2); }\n}\n",
    )
    .unwrap();

    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    install(&project_dir, false, true)
        .await
        .expect("install should succeed");
    let result = test(&project_dir, no_options()).await;

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(matches!(result, Err(CliError::TestsFailed(1))));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn test_filters_by_tag_and_excludes_non_matching_tests() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("tagged");
    let home_dir = temp_dir("tagged-home");

    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
                     [repositories]\ndefault = \"https://repo1.maven.org/maven2\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/test/java")).unwrap();
    std::fs::write(
        project_dir.join("src/test/java/MixedTest.java"),
        "import org.junit.jupiter.api.Test;\n\
         import org.junit.jupiter.api.Tag;\n\
         import static org.junit.jupiter.api.Assertions.assertEquals;\n\
         import static org.junit.jupiter.api.Assertions.fail;\n\
         class MixedTest {\n\
         \x20   @Test @Tag(\"fast\") void fastOne() { assertEquals(1, 1); }\n\
         \x20   @Test void slowAndBroken() { fail(\"never selected\"); }\n\
         }\n",
    )
    .unwrap();

    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    install(&project_dir, false, true)
        .await
        .expect("install should succeed");
    let result = test(
        &project_dir,
        TestOptions {
            filter: Some("tag:fast".to_string()),
            fail_fast: false,
            report_xml: false,
        },
    )
    .await;

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    result.expect("filtered run should only select the fast, passing test");

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn test_rejects_fail_fast_as_not_supported() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("fail-fast");
    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();

    let result = test(
        &project_dir,
        TestOptions {
            filter: None,
            fail_fast: true,
            report_xml: false,
        },
    )
    .await;

    assert!(matches!(result, Err(CliError::FailFastNotSupported)));

    let _ = std::fs::remove_dir_all(&project_dir);
}

/// Diferente do teste "contra Maven Central real" acima, este exercita
/// `devdeps::resolve_dev_classpath` (resolução + download de
/// `[dev-dependencies]`) contra o mesmo tipo de mock HTTP local que
/// `tests/cli_install.rs` já usa pra dependências de produção — prova que
/// a resolução de dev-deps reaproveita esse pipeline de verdade, não só
/// por leitura de código.
#[tokio::test]
async fn test_resolves_and_downloads_dev_dependencies() {
    let _guard = HOME_GUARD.lock().await;

    let jar_bytes = build_fake_dependency_jar(
        "Helper",
        "public class Helper { public static String shout(String s) { return s.toUpperCase(); } }",
    );
    let sha256 = hash_bytes(&jar_bytes);
    const EMPTY_POM: &str = "<project><dependencies></dependencies></project>";

    let jar_bytes_clone = jar_bytes.clone();
    let sha256_clone = sha256.clone();
    let server = support::start_mock_server(move |path| match path {
        "/com/acme/helper/1.0.0/helper-1.0.0.pom" => (200, EMPTY_POM.as_bytes().to_vec()),
        "/com/acme/helper/1.0.0/helper-1.0.0.jar" => (200, jar_bytes_clone.clone()),
        "/com/acme/helper/1.0.0/helper-1.0.0.jar.sha256" => {
            (200, sha256_clone.clone().into_bytes())
        }
        _ => (404, Vec::new()),
    });

    let project_dir = temp_dir("dev-deps");
    let home_dir = temp_dir("dev-deps-home");

    let manifest = format!(
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n\n\
         [dev-dependencies]\n\"com.acme:helper\" = \"1.0.0\"\n\n\
         [repositories]\ndefault = \"{}\"\n",
        server.base_url
    );
    std::fs::write(project_dir.join("project.toml"), &manifest).unwrap();
    std::fs::create_dir_all(project_dir.join("src/test/java")).unwrap();
    std::fs::write(
        project_dir.join("src/test/java/DevDepTest.java"),
        "import org.junit.jupiter.api.Test;\n\
         import static org.junit.jupiter.api.Assertions.assertEquals;\n\
         class DevDepTest {\n    @Test void usesHelper() { assertEquals(\"HI\", Helper.shout(\"hi\")); }\n}\n",
    )
    .unwrap();

    install_fake_jdk(&home_dir.join(".cache/jvmfast/jdks"), "21.0.1-tem");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    install(&project_dir, false, true)
        .await
        .expect("install should succeed");
    let result = test(&project_dir, no_options()).await;

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    let message = result.expect("test using a dev-dependency should compile and pass");
    assert!(message.contains("all tests passed"));

    let _ = std::fs::remove_dir_all(&project_dir);
    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn test_rejects_missing_lockfile() {
    let _guard = HOME_GUARD.lock().await;

    let project_dir = temp_dir("no-lock");
    let manifest = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\njava-version = \"21\"\n";
    std::fs::write(project_dir.join("project.toml"), manifest).unwrap();

    let result = test(&project_dir, no_options()).await;

    assert!(matches!(result, Err(CliError::LockfileMissing)));

    let _ = std::fs::remove_dir_all(&project_dir);
}
