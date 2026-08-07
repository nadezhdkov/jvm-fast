mod support;

use jvmfast::cache::{hash_bytes, CacheStore};
use jvmfast::domain::NetworkConfig;
use jvmfast::download::{ArtifactRequest, DownloadClient, DownloadError, PublishedChecksum};
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

#[tokio::test]
async fn fetch_checksum_reads_first_token_of_sha256_sidecar() {
    let server = start_mock_server(|path| {
        if path == "/demo-1.0.0.jar.sha256" {
            (200, b"ABCDEF0123  demo-1.0.0.jar\n".to_vec())
        } else {
            (404, Vec::new())
        }
    });

    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let checksum = client
        .fetch_checksum(&format!("{}/demo-1.0.0.jar", server.base_url))
        .await
        .expect("should fetch checksum");

    assert_eq!(
        checksum,
        PublishedChecksum::Sha256("abcdef0123".to_string())
    );
}

/// Maven Central real nem sempre publica `.sha256` (confirmado contra
/// artefatos reais como `slf4j-api`/`guava`/`hamcrest` — só `.sha1` é
/// universal na prática) — `fetch_checksum` precisa cair pro `.sha1`
/// quando o `.sha256` responder 404, em vez de falhar.
#[tokio::test]
async fn fetch_checksum_falls_back_to_sha1_when_sha256_sidecar_is_missing() {
    let server = start_mock_server(|path| {
        if path == "/demo-1.0.0.jar.sha1" {
            (200, b"1234567890abcdef".to_vec())
        } else {
            (404, Vec::new())
        }
    });

    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let checksum = client
        .fetch_checksum(&format!("{}/demo-1.0.0.jar", server.base_url))
        .await
        .expect("should fall back to sha1");

    assert_eq!(
        checksum,
        PublishedChecksum::Sha1("1234567890abcdef".to_string())
    );
}

#[tokio::test]
async fn fetch_checksum_fails_when_neither_sha256_nor_sha1_exists() {
    let server = start_mock_server(|_path| (404, Vec::new()));

    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let result = client
        .fetch_checksum(&format!("{}/demo-1.0.0.jar", server.base_url))
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::Status { status: 404, .. })
    ));
}

#[tokio::test]
async fn fetch_verify_and_cache_uses_sha256_sidecar_and_skips_download_when_already_cached() {
    let contents = b"already cached bytes".to_vec();
    let sha256 = hash_bytes(&contents);
    let sha256_for_server = sha256.clone();
    let server = start_mock_server(move |path| {
        if path == "/demo-1.0.0.jar.sha256" {
            (200, sha256_for_server.clone().into_bytes())
        } else {
            (404, Vec::new())
        }
    });

    let dir = temp_dir("fetch-verify-cached");
    let store = CacheStore::new(&dir);
    store
        .write_artifact(&contents, &sha256, "demo-1.0.0.jar")
        .expect("should seed cache");

    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let resolved = client
        .fetch_verify_and_cache(
            &format!("{}/demo-1.0.0.jar", server.base_url),
            "demo-1.0.0.jar",
            &store,
        )
        .await
        .expect("should resolve from cache without downloading the jar itself");

    assert_eq!(resolved.sha256, sha256);
    assert!(resolved.reused_from_cache);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn fetch_verify_and_cache_downloads_and_computes_sha256_when_only_sha1_is_published() {
    let contents = b"only a sha1 sidecar exists for this one".to_vec();
    let sha1 = {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(&contents);
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let real_sha256 = hash_bytes(&contents);

    let contents_clone = contents.clone();
    let sha1_clone = sha1.clone();
    let server = start_mock_server(move |path| match path {
        "/demo-1.0.0.jar.sha1" => (200, sha1_clone.clone().into_bytes()),
        "/demo-1.0.0.jar" => (200, contents_clone.clone()),
        _ => (404, Vec::new()),
    });

    let dir = temp_dir("fetch-verify-sha1");
    let store = CacheStore::new(&dir);
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");

    let resolved = client
        .fetch_verify_and_cache(
            &format!("{}/demo-1.0.0.jar", server.base_url),
            "demo-1.0.0.jar",
            &store,
        )
        .await
        .expect("should download, verify against sha1, and cache under the real sha256");

    assert_eq!(resolved.sha256, real_sha256);
    assert!(!resolved.reused_from_cache);
    assert!(store.is_cached(&real_sha256, "demo-1.0.0.jar"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn fetch_verify_and_cache_rejects_sha1_mismatch() {
    let contents = b"tampered content".to_vec();
    let server = start_mock_server(move |path| match path {
        "/demo-1.0.0.jar.sha1" => (200, b"0000000000000000000000000000000000000000".to_vec()),
        "/demo-1.0.0.jar" => (200, contents.clone()),
        _ => (404, Vec::new()),
    });

    let dir = temp_dir("fetch-verify-sha1-mismatch");
    let store = CacheStore::new(&dir);
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");

    let result = client
        .fetch_verify_and_cache(
            &format!("{}/demo-1.0.0.jar", server.base_url),
            "demo-1.0.0.jar",
            &store,
        )
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::ChecksumMismatch {
            algorithm: "sha1",
            ..
        })
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_verify_and_cache_many_resolves_every_item_and_preserves_keys() {
    let contents_a = b"artifact-a-bytes".to_vec();
    let contents_b = b"artifact-b-bytes".to_vec();
    let sha_a = hash_bytes(&contents_a);
    let sha_b = hash_bytes(&contents_b);
    let sha_a_for_server = sha_a.clone();
    let sha_b_for_server = sha_b.clone();

    let server = start_mock_server(move |path| match path {
        "/a.jar.sha256" => (200, sha_a_for_server.clone().into_bytes()),
        "/b.jar.sha256" => (200, sha_b_for_server.clone().into_bytes()),
        "/a.jar" => (200, contents_a.clone()),
        "/b.jar" => (200, contents_b.clone()),
        _ => (404, Vec::new()),
    });

    let dir = temp_dir("fetch-verify-many");
    let store = std::sync::Arc::new(CacheStore::new(&dir));
    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");

    let items = vec![
        (
            "a".to_string(),
            format!("{}/a.jar", server.base_url),
            "a.jar".to_string(),
        ),
        (
            "b".to_string(),
            format!("{}/b.jar", server.base_url),
            "b.jar".to_string(),
        ),
    ];

    let results = client
        .fetch_verify_and_cache_many(items, std::sync::Arc::clone(&store), 2)
        .await;

    let by_key: std::collections::HashMap<_, _> = results.into_iter().collect();
    assert_eq!(by_key.get("a").unwrap().as_ref().unwrap().sha256, sha_a);
    assert_eq!(by_key.get("b").unwrap().as_ref().unwrap().sha256, sha_b);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn fetch_checksum_rejects_empty_sidecar_body() {
    let server = start_mock_server(|_path| (200, Vec::new()));

    let client = DownloadClient::new(&NetworkConfig::default()).expect("should build client");
    let result = client
        .fetch_checksum(&format!("{}/demo-1.0.0.jar", server.base_url))
        .await;

    assert!(matches!(result, Err(DownloadError::EmptyChecksum { .. })));
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
