mod error;

pub use error::DownloadError;

use crate::cache::CacheStore;
use crate::domain::NetworkConfig;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Um artefato a baixar: URL completa do repositório, nome de arquivo para
/// o cache e o checksum esperado (do lockfile já existente, ou de metadata
/// do repositório quando o lock está sendo gerado agora — seção 6.2 passo 7).
#[derive(Debug, Clone)]
pub struct ArtifactRequest {
    pub url: String,
    pub filename: String,
    pub expected_sha256: String,
}

/// Cliente HTTP para o passo 6 da seção 6.2 ("Download paralelo"). Esta é a
/// primeira superfície `async` do projeto — justificada porque
/// `download_many` precisa de concorrência real; `pom::HttpPomProvider`
/// (fetch de POM durante a resolução do grafo, inerentemente sequencial)
/// permanece síncrono de propósito, por CONVENTIONS.md: "não colocar async
/// por hábito".
pub struct DownloadClient {
    client: reqwest::Client,
    max_retries: u32,
}

impl DownloadClient {
    pub fn new(network: &NetworkConfig) -> Result<Self, DownloadError> {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(u64::from(network.connect_timeout_secs)));

        if let Some(proxy_url) = &network.proxy {
            let proxy = reqwest::Proxy::all(proxy_url).map_err(DownloadError::ClientBuild)?;
            builder = builder.proxy(proxy);
        }

        let client = builder.build().map_err(DownloadError::ClientBuild)?;
        Ok(Self {
            client,
            max_retries: network.max_retries,
        })
    }

    async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, DownloadError> {
        let attempts = self.max_retries.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.client.get(url).send().await {
                Ok(response) if response.status().is_success() => {
                    return response
                        .bytes()
                        .await
                        .map(|b| b.to_vec())
                        .map_err(|source| DownloadError::Request {
                            url: url.to_string(),
                            attempts: attempt,
                            source,
                        });
                }
                Ok(response) => {
                    last_error = Some(DownloadError::Status {
                        url: url.to_string(),
                        status: response.status().as_u16(),
                    });
                }
                Err(source) => {
                    last_error = Some(DownloadError::Request {
                        url: url.to_string(),
                        attempts: attempt,
                        source,
                    });
                }
            }
        }

        Err(last_error.expect("the loop above runs at least once, since attempts >= 1"))
    }

    /// Baixa um único artefato e o persiste no cache — `CacheStore::write_artifact`
    /// já cobre a verificação de checksum e o rename atômico (seção 5.1);
    /// este método só cuida da parte de rede.
    pub async fn download_artifact(
        &self,
        request: &ArtifactRequest,
        store: &CacheStore,
    ) -> Result<PathBuf, DownloadError> {
        let contents = self.fetch_bytes(&request.url).await?;
        store
            .write_artifact(&contents, &request.expected_sha256, &request.filename)
            .map_err(DownloadError::from)
    }

    /// Download paralelo (seção 6.2 passo 6), com pool limitado a
    /// `max_concurrent` (seção 3.5: `network.concurrent_downloads`, default
    /// = número de cores). A arquitetura também documenta limite *por
    /// repositório host* — como `ArtifactRequest` ainda não carrega
    /// identidade de repositório (o parsing de `[repositories]` não é
    /// thread-ado até `Module` ainda, gap sinalizado em
    /// `src/manifest/dto.rs`), esta passada limita globalmente; o limite
    /// por host fica para quando repositórios nomeados alcançarem o fluxo
    /// de resolução de verdade.
    pub async fn download_many(
        &self,
        requests: Vec<ArtifactRequest>,
        store: Arc<CacheStore>,
        max_concurrent: usize,
    ) -> Vec<Result<PathBuf, DownloadError>> {
        let semaphore = Arc::new(Semaphore::new(max_concurrent.max(1)));
        let mut tasks = tokio::task::JoinSet::new();

        for request in requests {
            let semaphore = Arc::clone(&semaphore);
            let store = Arc::clone(&store);
            let client = self.client.clone();
            let max_retries = self.max_retries;

            tasks.spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("semaphore is never closed while tasks are running");
                let download_client = DownloadClient {
                    client,
                    max_retries,
                };
                download_client.download_artifact(&request, &store).await
            });
        }

        let mut results = Vec::new();
        while let Some(joined) = tasks.join_next().await {
            results.push(joined.expect("download task should never panic"));
        }
        results
    }
}
