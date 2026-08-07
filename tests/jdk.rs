mod support;

use flate2::write::GzEncoder;
use flate2::Compression;
use jvmfast::cache::hash_bytes;
use jvmfast::jdk::{install, is_installed, list_installed, parse_major_version, AdoptiumClient};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("jvmfast-test-jdk-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

/// Monta um `.tar.gz` mínimo com um único diretório-raiz contendo
/// `bin/java`, imitando a forma real de um tarball Temurin o suficiente
/// para exercitar `jdk::install` sem baixar nada real.
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

/// Servidor mock que serve tanto a resposta JSON da API do Adoptium quanto
/// o `.tar.gz` referenciado nela — o corpo JSON precisa embutir a própria
/// `base_url` do servidor, que só existe depois que ele já subiu, daí o
/// `OnceLock` preenchido logo em seguida.
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

#[tokio::test]
async fn adoptium_client_parses_latest_release_response() {
    let archive = fake_jdk_tar_gz("jdk-21.0.2+13");
    let checksum = hash_bytes(&archive);
    let server = start_adoptium_mock(archive, checksum.clone());

    let adoptium = AdoptiumClient::new(server.base_url.clone());
    let release = adoptium
        .latest_release("21", "linux", "x64")
        .await
        .expect("should fetch and parse release");

    assert_eq!(release.version, "21.0.2");
    assert_eq!(release.checksum, checksum);
    assert_eq!(
        release.download_url,
        format!("{}/download/jdk.tar.gz", server.base_url)
    );
}

#[tokio::test]
async fn install_downloads_verifies_and_extracts_jdk_archive() {
    let archive = fake_jdk_tar_gz("jdk-21.0.2+13");
    let checksum = hash_bytes(&archive);
    let server = start_adoptium_mock(archive, checksum);

    let adoptium = AdoptiumClient::new(server.base_url.clone());
    let release = adoptium
        .latest_release("21", "linux", "x64")
        .await
        .expect("should fetch release");

    let jdks_root = temp_dir("install");
    let client = reqwest::Client::new();
    let installed_path = install(&client, &jdks_root, &release)
        .await
        .expect("should install");

    assert_eq!(installed_path, jdks_root.join("21.0.2-tem"));
    assert!(installed_path.join("bin/java").is_file());
    assert!(is_installed(&jdks_root, &release));

    let installed = list_installed(&jdks_root).expect("should list");
    assert_eq!(installed, vec!["21.0.2-tem".to_string()]);

    let _ = std::fs::remove_dir_all(&jdks_root);
}

#[tokio::test]
async fn install_is_idempotent_when_already_installed() {
    let archive = fake_jdk_tar_gz("jdk-21.0.2+13");
    let checksum = hash_bytes(&archive);
    let server = start_adoptium_mock(archive, checksum);

    let adoptium = AdoptiumClient::new(server.base_url.clone());
    let release = adoptium
        .latest_release("21", "linux", "x64")
        .await
        .expect("should fetch release");

    let jdks_root = temp_dir("idempotent");
    let client = reqwest::Client::new();
    let first = install(&client, &jdks_root, &release)
        .await
        .expect("should install");
    let second = install(&client, &jdks_root, &release)
        .await
        .expect("should be a no-op the second time");

    assert_eq!(first, second);

    let _ = std::fs::remove_dir_all(&jdks_root);
}

#[tokio::test]
async fn install_rejects_checksum_mismatch() {
    let archive = fake_jdk_tar_gz("jdk-21.0.2+13");
    // checksum errado de propósito
    let server = start_adoptium_mock(archive, "0".repeat(64));

    let adoptium = AdoptiumClient::new(server.base_url.clone());
    let release = adoptium
        .latest_release("21", "linux", "x64")
        .await
        .expect("should fetch release");

    let jdks_root = temp_dir("mismatch");
    let client = reqwest::Client::new();
    let result = install(&client, &jdks_root, &release).await;

    assert!(matches!(
        result,
        Err(jvmfast::jdk::JdkError::ChecksumMismatch { .. })
    ));
    assert!(!is_installed(&jdks_root, &release));

    let _ = std::fs::remove_dir_all(&jdks_root);
}

#[test]
fn list_installed_returns_empty_when_root_absent() {
    let jdks_root = temp_dir("absent").join("does-not-exist");

    let installed = list_installed(&jdks_root).expect("should not error");

    assert!(installed.is_empty());
}

#[test]
fn parse_major_version_accepts_digits_only() {
    assert_eq!(parse_major_version("21").unwrap(), "21");
}

#[test]
fn parse_major_version_rejects_exact_pinned_version() {
    let result = parse_major_version("21.0.2-tem");

    assert!(matches!(
        result,
        Err(jvmfast::jdk::JdkError::ExactVersionNotSupported(_))
    ));
}
