mod error;

pub use error::DownloadError;

use crate::cache::{hash_bytes, CacheStore};
use crate::domain::NetworkConfig;
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Um artefato a baixar: URL completa do repositório, nome de arquivo para
/// o cache e o checksum SHA-256 esperado — sempre um valor já conhecido e
/// confiável (do lockfile existente), nunca algo que ainda precise de
/// verificação em cascata (`fetch_verify_and_cache*` cobre esse caso, ver
/// abaixo). `DownloadClient::download_many` é o caminho da seção 6.2 passo
/// 6 quando o `project.lock` já diz exatamente qual SHA-256 cada artefato
/// deve ter.
#[derive(Debug, Clone)]
pub struct ArtifactRequest {
    pub url: String,
    pub filename: String,
    pub expected_sha256: String,
}

/// Checksum publicado pelo repositório para um artefato (seção 6.2 passo
/// 7) — `.sha256` é preferido, mas o Maven Central real nem sempre publica
/// um (confirmado contra `slf4j-api`/`guava`/`hamcrest`, entre outros
/// artefatos comuns: só têm `.sha1`/`.md5`, `.sha256` é recente e
/// opcional). `Sha1` existe só para verificar a integridade do download —
/// a identidade do artefato no cache/lockfile deste projeto é sempre
/// SHA-256 (seção 5), nunca o hash publicado diretamente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedChecksum {
    Sha256(String),
    Sha1(String),
}

impl PublishedChecksum {
    fn algorithm_name(&self) -> &'static str {
        match self {
            Self::Sha256(_) => "sha256",
            Self::Sha1(_) => "sha1",
        }
    }
}

/// Um artefato já baixado, verificado e persistido no cache — o SHA-256 é
/// sempre o hash real do conteúdo (`cache::hash_bytes`), nunca o valor
/// publicado pelo repositório quando esse valor não era SHA-256 (ver
/// `PublishedChecksum::Sha1`).
#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub path: PathBuf,
    pub sha256: String,
    pub reused_from_cache: bool,
}

fn hash_sha1(contents: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(contents);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
                // 404 é definitivo, não transitório — repetir não muda o
                // resultado, então retorna direto em vez de gastar as
                // outras tentativas (importante para o fallback
                // sha256→sha1 de `fetch_checksum`, que depende de um 404
                // rápido para saber que deve tentar o sidecar seguinte).
                Ok(response) if response.status().as_u16() == 404 => {
                    return Err(DownloadError::Status {
                        url: url.to_string(),
                        status: 404,
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

    async fn fetch_sidecar(
        &self,
        artifact_url: &str,
        extension: &str,
    ) -> Result<String, DownloadError> {
        let sidecar_url = format!("{artifact_url}.{extension}");
        let contents = self.fetch_bytes(&sidecar_url).await?;
        let text = String::from_utf8_lossy(&contents);
        text.split_whitespace()
            .next()
            .map(str::to_lowercase)
            .filter(|hash| !hash.is_empty())
            .ok_or(DownloadError::EmptyChecksum { url: sidecar_url })
    }

    /// Busca o checksum publicado pelo repositório para um artefato ainda
    /// não presente no lockfile (seção 6.2 passo 7: "SHA-256... comparado
    /// contra o valor do lockfile, **ou do repositório, se o lock está
    /// sendo gerado agora**"). Prefere `.sha256`; se o repositório
    /// responder 404 pra ele (comum no Maven Central real — ver
    /// `PublishedChecksum`), cai para `.sha1`, que é praticamente
    /// universal. Erros que não sejam 404 (rede, 5xx, etc.) propagam sem
    /// tentar o fallback — só a ausência confirmada do sidecar preferido
    /// justifica cair pro próximo.
    pub async fn fetch_checksum(
        &self,
        artifact_url: &str,
    ) -> Result<PublishedChecksum, DownloadError> {
        match self.fetch_sidecar(artifact_url, "sha256").await {
            Ok(value) => Ok(PublishedChecksum::Sha256(value)),
            Err(DownloadError::Status { status: 404, .. }) => self
                .fetch_sidecar(artifact_url, "sha1")
                .await
                .map(PublishedChecksum::Sha1),
            Err(other) => Err(other),
        }
    }

    /// Baixa um único artefato e o persiste no cache — `CacheStore::write_artifact`
    /// já cobre a verificação de checksum e o rename atômico (seção 5.1);
    /// este método só cuida da parte de rede. Assume que `request.expected_sha256`
    /// já é um SHA-256 confiável (do lockfile) — para o caso "ainda não sei
    /// o SHA-256, só tenho o que o repositório publicou", ver
    /// `fetch_verify_and_cache`.
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

    /// Resolve um artefato do zero, sem nenhum SHA-256 pré-conhecido —
    /// caminho usado quando o lock está sendo gerado agora e a única fonte
    /// de verdade é o que o repositório publica (`fetch_checksum`, seção
    /// 6.2 passo 7). Se o sidecar for `.sha256`, o caminho rápido de
    /// sempre (seção 5: já em cache pelo hash → nem baixa de novo) se
    /// aplica normalmente. Se só houver `.sha1` publicado, não dá pra
    /// checar o cache antes de baixar (o cache é indexado por SHA-256, que
    /// só é conhecido depois de baixar e hashear o conteúdo) — então esse
    /// caso sempre baixa, verifica contra o SHA-1 publicado, e só então
    /// calcula e grava sob o SHA-256 real. Gap aceito, documentado em
    /// CLAUDE.md: artefatos só-com-SHA-1 perdem o atalho de cache na
    /// primeira resolução (mas não depois — uma vez no lockfile, o SHA-256
    /// real já fica pinado e `download_many`/`ArtifactRequest` cobrem o
    /// resto normalmente).
    pub async fn fetch_verify_and_cache(
        &self,
        url: &str,
        filename: &str,
        store: &CacheStore,
    ) -> Result<ResolvedArtifact, DownloadError> {
        let checksum = self.fetch_checksum(url).await?;

        if let PublishedChecksum::Sha256(sha256) = &checksum {
            if store.is_cached(sha256, filename) {
                return Ok(ResolvedArtifact {
                    path: store.artifact_path(sha256, filename),
                    sha256: sha256.clone(),
                    reused_from_cache: true,
                });
            }
        }

        let contents = self.fetch_bytes(url).await?;
        let sha256 = match &checksum {
            PublishedChecksum::Sha256(expected) => {
                let actual = hash_bytes(&contents);
                if &actual != expected {
                    return Err(DownloadError::ChecksumMismatch {
                        url: url.to_string(),
                        algorithm: checksum.algorithm_name(),
                        expected: expected.clone(),
                        actual,
                    });
                }
                actual
            }
            PublishedChecksum::Sha1(expected) => {
                let actual = hash_sha1(&contents);
                if &actual != expected {
                    return Err(DownloadError::ChecksumMismatch {
                        url: url.to_string(),
                        algorithm: checksum.algorithm_name(),
                        expected: expected.clone(),
                        actual,
                    });
                }
                hash_bytes(&contents)
            }
        };

        let path = store.write_artifact(&contents, &sha256, filename)?;
        Ok(ResolvedArtifact {
            path,
            sha256,
            reused_from_cache: false,
        })
    }

    /// Versão paralela de `fetch_verify_and_cache` (mesma disciplina de
    /// concorrência de `download_many`) — `key` é um identificador opaco
    /// escolhido pelo chamador (ex. `"coordenada@versão"`) pra recolocar
    /// cada resultado no lugar certo depois, já que a ordem de conclusão
    /// não é a ordem de entrada.
    pub async fn fetch_verify_and_cache_many(
        &self,
        items: Vec<(String, String, String)>,
        store: Arc<CacheStore>,
        max_concurrent: usize,
    ) -> Vec<(String, Result<ResolvedArtifact, DownloadError>)> {
        let semaphore = Arc::new(Semaphore::new(max_concurrent.max(1)));
        let mut tasks = tokio::task::JoinSet::new();

        for (key, url, filename) in items {
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
                let result = download_client
                    .fetch_verify_and_cache(&url, &filename, &store)
                    .await;
                (key, result)
            });
        }

        let mut results = Vec::new();
        while let Some(joined) = tasks.join_next().await {
            results.push(joined.expect("download task should never panic"));
        }
        results
    }
}
