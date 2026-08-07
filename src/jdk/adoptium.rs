use super::error::JdkError;
use serde::Deserialize;

/// Um release de JDK resolvido — o suficiente para baixar, verificar e
/// instalar (seção 7). `version` é `major.minor.security` (ex. `"21.0.2"`),
/// usado para nomear o diretório de instalação (`<version>-tem`, seção 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JdkRelease {
    pub version: String,
    pub download_url: String,
    pub checksum: String,
    pub filename: String,
}

#[derive(Deserialize)]
struct AssetEntry {
    binary: Binary,
    version: VersionInfo,
}

#[derive(Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    link: String,
    checksum: String,
    name: String,
}

#[derive(Deserialize)]
struct VersionInfo {
    major: u32,
    minor: u32,
    security: u32,
}

#[derive(Deserialize)]
struct AvailableReleases {
    most_recent_lts: u32,
}

/// Cliente da API pública do Eclipse Temurin/Adoptium
/// (`https://api.adoptium.net`) — a única distribuição que a seção 7
/// documenta ("Distribuição padrão: Eclipse Temurin"). O formato da
/// resposta é assumido a partir da documentação pública da API v3, não
/// verificado contra produção neste ambiente (sem acesso à rede externa) —
/// testado aqui contra um fixture JSON local, nunca contra
/// `api.adoptium.net` de verdade, seguindo a mesma regra de
/// CONVENTIONS.md já aplicada a `pom::HttpPomProvider`.
pub struct AdoptiumClient {
    client: reqwest::Client,
    base_url: String,
}

impl AdoptiumClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// GET `/v3/assets/latest/{feature_version}/hotspot` — a release mais
    /// recente da JVM `hotspot` para a major version pedida, filtrada por
    /// `os`/`arch` (valores já no vocabulário do Adoptium — ver
    /// `jdk::current_platform`).
    pub async fn latest_release(
        &self,
        feature_version: &str,
        os: &str,
        arch: &str,
    ) -> Result<JdkRelease, JdkError> {
        let url = format!(
            "{}/v3/assets/latest/{feature_version}/hotspot?architecture={arch}&image_type=jdk&os={os}",
            self.base_url.trim_end_matches('/'),
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|source| JdkError::Request {
                url: url.clone(),
                source,
            })?;

        if !response.status().is_success() {
            return Err(JdkError::Status {
                url,
                status: response.status().as_u16(),
            });
        }

        let body = response.text().await.map_err(|source| JdkError::Request {
            url: url.clone(),
            source,
        })?;

        let entries: Vec<AssetEntry> =
            serde_json::from_str(&body).map_err(|source| JdkError::Parse { url, source })?;
        let entry = entries
            .into_iter()
            .next()
            .ok_or_else(|| JdkError::NoReleaseFound(feature_version.to_string()))?;

        Ok(JdkRelease {
            version: format!(
                "{}.{}.{}",
                entry.version.major, entry.version.minor, entry.version.security
            ),
            download_url: entry.binary.package.link,
            checksum: entry.binary.package.checksum,
            filename: entry.binary.package.name,
        })
    }

    /// GET `/v3/info/available_releases` — usado só para resolver o alias
    /// `"lts"` (seção 3) para uma major version concreta. Assumido a partir
    /// da documentação pública da API v3, mesma ressalva de honestidade que
    /// `latest_release`.
    pub async fn most_recent_lts(&self) -> Result<u32, JdkError> {
        let url = format!(
            "{}/v3/info/available_releases",
            self.base_url.trim_end_matches('/'),
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|source| JdkError::Request {
                url: url.clone(),
                source,
            })?;

        if !response.status().is_success() {
            return Err(JdkError::Status {
                url,
                status: response.status().as_u16(),
            });
        }

        let body = response.text().await.map_err(|source| JdkError::Request {
            url: url.clone(),
            source,
        })?;

        let releases: AvailableReleases =
            serde_json::from_str(&body).map_err(|source| JdkError::Parse { url, source })?;
        Ok(releases.most_recent_lts)
    }
}
