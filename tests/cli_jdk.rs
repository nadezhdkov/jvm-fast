mod support;

use flate2::write::GzEncoder;
use flate2::Compression;
use jvmfast::cache::hash_bytes;
use jvmfast::cli::CliError;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

/// `jdk::install_jdk`/`jdk::list_jdks`/`jdk::use_jdk` todos resolvem raízes
/// a partir de `$HOME` (`cli::context::{jdks_root, config_path}`) — mutar
/// essa env var globalmente exige serializar os testes deste arquivo, como
/// já feito em `tests/cli_install.rs`.
static HOME_GUARD: Mutex<()> = Mutex::const_new(());

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-cli-jdk-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn fake_jdk_tar_gz(root_dir_name: &str) -> Vec<u8> {
    let file_contents = b"#!/bin/sh\necho fake java\n";
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(file_contents.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(
            &mut header,
            format!("{root_dir_name}/bin/java"),
            &file_contents[..],
        )
        .expect("should append tar entry");
    let tar_bytes = builder.into_inner().expect("should finish tar");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_bytes).expect("should gzip");
    encoder.finish().expect("should finish gzip")
}

fn start_adoptium_mock(archive: Vec<u8>, checksum: String) -> support::MockHttpServer {
    let base_url_holder: Arc<OnceLock<String>> = Arc::new(OnceLock::new());
    let base_url_for_closure = Arc::clone(&base_url_holder);

    let server = support::start_mock_server(move |path| {
        if path.starts_with("/v3/assets/latest/21/hotspot") {
            let base = base_url_for_closure.get().cloned().unwrap_or_default();
            let body = format!(
                r#"[{{"binary":{{"package":{{"link":"{base}/download/jdk.tar.gz","checksum":"{checksum}","name":"OpenJDK21U-jdk_x64_linux_hotspot_21.0.2_13.tar.gz"}}}},"version":{{"major":21,"minor":0,"security":2}}}}]"#
            );
            (200, body.into_bytes())
        } else if path == "/download/jdk.tar.gz" {
            (200, archive.clone())
        } else {
            (404, Vec::new())
        }
    });
    base_url_holder
        .set(server.base_url.clone())
        .expect("should set base url exactly once");
    server
}

/// Este teste não pode passar por `jdk::install_jdk` (que aponta pra API
/// real do Adoptium via `cli::context::ADOPTIUM_API`, sem jeito de apontar
/// pro mock a partir do comando público) — em vez disso, instala direto
/// via `jvmfast::jdk::install` (mesma função de baixo nível que
/// `cli::jdk::install_jdk` chama) contra o mock, e exercita só
/// `list_jdks`/`use_jdk` (que são públicos e realistas de testar) por cima
/// do resultado.
#[tokio::test]
async fn use_jdk_sets_default_and_list_marks_it() {
    let _guard = HOME_GUARD.lock().await;

    let archive = fake_jdk_tar_gz("jdk-21.0.2+13");
    let checksum = hash_bytes(&archive);
    let server = start_adoptium_mock(archive, checksum);

    let home_dir = temp_dir("home");
    let jdks_root = home_dir.join(".cache/jvmfast/jdks");

    let adoptium = jvmfast::jdk::AdoptiumClient::new(server.base_url.clone());
    let release = adoptium
        .latest_release("21", "linux", "x64")
        .await
        .expect("should fetch release");
    let client = reqwest::Client::new();
    jvmfast::jdk::install(&client, &jdks_root, &release)
        .await
        .expect("should install into fake HOME");

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let list_before = jvmfast::cli::list_jdks().expect("should list");
    assert_eq!(list_before, "21.0.2-tem");

    let use_result = jvmfast::cli::use_jdk("21").expect("should set default");
    assert!(use_result.contains("21"));

    let list_after = jvmfast::cli::list_jdks().expect("should list");
    assert_eq!(list_after, "21.0.2-tem (default)");

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn use_jdk_rejects_version_not_installed() {
    let _guard = HOME_GUARD.lock().await;

    let home_dir = temp_dir("home-empty");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let result = jvmfast::cli::use_jdk("21");

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(matches!(
        result,
        Err(CliError::JavaVersionNotInstalled(version)) if version == "21"
    ));

    let _ = std::fs::remove_dir_all(&home_dir);
}

#[tokio::test]
async fn list_jdks_reports_no_jdks_installed_when_empty() {
    let _guard = HOME_GUARD.lock().await;

    let home_dir = temp_dir("home-list-empty");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home_dir);

    let result = jvmfast::cli::list_jdks().expect("should not error");

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    assert!(result.contains("no JDKs installed"));

    let _ = std::fs::remove_dir_all(&home_dir);
}
