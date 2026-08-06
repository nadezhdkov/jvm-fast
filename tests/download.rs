mod support;

use jvmfast::cache::{hash_bytes, CacheStore};
use jvmfast::domain::NetworkConfig;
use jvmfast::download::{ArtifactRequest, DownloadClient, DownloadError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use support::start_mock_server;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jvmfast-test-download-{}-{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

#[tokio::test]
async fn download_artifact_writes_verified_content_to_cache() {
    let contents = b"fake jar bytes".to_vec();
    let sha256 = hash_bytes(&contents);
    let server = start_mock_server(move |_path| (200, contents.clone()));

    let dir = temp_dir("single");
    let store = CacheStore::new(&dir);
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let request = ArtifactRequest {
        url: format!("{}/demo-1.0.0.jar", server.base_url),
        filename: "demo-1.0.0.jar".to_string(),
        expected_sha256: sha256.clone(),
    };

    let path = client
        .download_artifact(&request, &store)
        .await
        .expect("should download and cache");

    assert!(path.is_file());
    assert!(store.is_cached(&sha256, "demo-1.0.0.jar"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn download_artifact_rejects_checksum_mismatch() {
    let server = start_mock_server(|_path| (200, b"unexpected content".to_vec()));

    let dir = temp_dir("mismatch");
    let store = CacheStore::new(&dir);
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let request = ArtifactRequest {
        url: format!("{}/demo-1.0.0.jar", server.base_url),
        filename: "demo-1.0.0.jar".to_string(),
        expected_sha256: "not-the-real-hash".to_string(),
    };

    let result = client.download_artifact(&request, &store).await;

    assert!(matches!(
        result,
        Err(DownloadError::Cache(
            jvmfast::cache::CacheError::ChecksumMismatch { .. }
        ))
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn download_artifact_returns_typed_error_on_http_status() {
    let server = start_mock_server(|_path| (404, Vec::new()));

    let dir = temp_dir("status");
    let store = CacheStore::new(&dir);
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let request = ArtifactRequest {
        url: format!("{}/missing.jar", server.base_url),
        filename: "missing.jar".to_string(),
        expected_sha256: "irrelevant".to_string(),
    };

    let result = client.download_artifact(&request, &store).await;

    assert!(matches!(
        result,
        Err(DownloadError::Status { status: 404, .. })
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn download_many_downloads_and_caches_every_artifact() {
    let contents_a = b"artifact-a-bytes".to_vec();
    let contents_b = b"artifact-b-bytes".to_vec();
    let sha_a = hash_bytes(&contents_a);
    let sha_b = hash_bytes(&contents_b);

    let server = start_mock_server(move |path| match path {
        "/a.jar" => (200, contents_a.clone()),
        "/b.jar" => (200, contents_b.clone()),
        _ => (404, Vec::new()),
    });

    let dir = temp_dir("many");
    let store = Arc::new(CacheStore::new(&dir));
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");

    let requests = vec![
        ArtifactRequest {
            url: format!("{}/a.jar", server.base_url),
            filename: "a.jar".to_string(),
            expected_sha256: sha_a.clone(),
        },
        ArtifactRequest {
            url: format!("{}/b.jar", server.base_url),
            filename: "b.jar".to_string(),
            expected_sha256: sha_b.clone(),
        },
    ];

    let results = client.download_many(requests, Arc::clone(&store), 2).await;

    assert!(results.iter().all(|r| r.is_ok()));
    assert!(store.is_cached(&sha_a, "a.jar"));
    assert!(store.is_cached(&sha_b, "b.jar"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn download_many_caps_concurrency_at_configured_limit() {
    let current = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let current_clone = Arc::clone(&current);
    let peak_clone = Arc::clone(&peak);

    let server = start_mock_server(move |_path| {
        let now = current_clone.fetch_add(1, Ordering::SeqCst) + 1;
        peak_clone.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(80));
        current_clone.fetch_sub(1, Ordering::SeqCst);
        (200, b"x".to_vec())
    });

    let sha = hash_bytes(b"x");
    let dir = temp_dir("concurrency-cap");
    let store = Arc::new(CacheStore::new(&dir));
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");

    let requests: Vec<_> = (0..6)
        .map(|i| ArtifactRequest {
            url: format!("{}/artifact-{i}.jar", server.base_url),
            filename: format!("artifact-{i}.jar"),
            expected_sha256: sha.clone(),
        })
        .collect();

    let results = client.download_many(requests, store, 2).await;

    assert!(results.iter().all(|r| r.is_ok()));
    assert!(
        peak.load(Ordering::SeqCst) <= 2,
        "peak concurrency should never exceed the configured limit"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
