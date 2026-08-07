use super::error::TestError;
use crate::cache::CacheStore;
use crate::domain::Module;
use crate::download::{ArtifactRequest, DownloadClient};
use crate::maven::{artifact_filename, artifact_url};
use crate::pom::HttpPomProvider;
use crate::resolve::resolve;
use std::path::PathBuf;
use std::sync::Arc;

/// Resolve e baixa `[dev-dependencies]` (seção 8.1) a partir do `Module`
/// sintético que `manifest::parse_dev_module` monta — roda a resolução
/// (BOMs → grafo → mediação, seção 6.2 passos 3–5) e o download inteiros a
/// cada `jvmfast test`, do mesmo jeito que `jvmfast install` roda quando
/// não existe `project.lock` nenhum ainda. **Gap deliberado**: diferente
/// das dependências de produção, isso nunca é cacheado num lockfile — o
/// schema de `Lockfile` (seção 4) não tem onde guardar um segundo grafo
/// resolvido para dev-deps ainda, então cada `test` re-resolve do zero
/// (mitigado pelo cache de artefatos em si: só o *grafo* é recalculado,
/// os `.jar` batidos permanecem em `~/.cache/jvmfast/`).
pub async fn resolve_dev_classpath(
    dev_module: &Module,
    base_url: &str,
    cache_store: Arc<CacheStore>,
    download_client: &DownloadClient,
    max_concurrent: usize,
) -> Result<Vec<PathBuf>, TestError> {
    let modules = vec![dev_module.clone()];
    let provider_base_url = base_url.to_string();
    // Mesma razão de `cli::install`/`cli::mod::resolve_for_diagnostics`:
    // `HttpPomProvider` usa `reqwest::blocking`, que não pode ser
    // construído nem usado de dentro de um runtime tokio já em execução.
    let resolve_output = tokio::task::spawn_blocking(move || {
        let provider = HttpPomProvider::new(provider_base_url);
        resolve(&modules, &provider)
    })
    .await??;

    let mut cached_paths = Vec::new();
    let mut requests = Vec::new();
    for node in resolve_output.graph.nodes.values() {
        let jar_url = artifact_url(base_url, &node.coordinate, &node.selected, "jar")?;
        let checksum = download_client.fetch_checksum(&jar_url).await?;
        let filename = artifact_filename(&node.coordinate, &node.selected, "jar")?;

        if cache_store.is_cached(&checksum, &filename) {
            cached_paths.push(cache_store.artifact_path(&checksum, &filename));
        } else {
            requests.push(ArtifactRequest {
                url: jar_url,
                filename,
                expected_sha256: checksum,
            });
        }
    }

    let results = download_client
        .download_many(requests, Arc::clone(&cache_store), max_concurrent)
        .await;

    let mut classpath = cached_paths;
    for result in results {
        classpath.push(result?);
    }
    Ok(classpath)
}
