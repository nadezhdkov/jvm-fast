use super::error::PomParseError;
use super::{parse_pom_xml, ParsedPom, PomProvider};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpPomError {
    #[error("invalid coordinate `{0}` — expected `groupId:artifactId`")]
    InvalidCoordinate(String),

    #[error("HTTP request for POM at `{url}` failed: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("unexpected HTTP status {status} fetching POM at `{url}`")]
    Status { url: String, status: u16 },

    #[error("failed to parse POM fetched from `{url}`: {source}")]
    Parse {
        url: String,
        #[source]
        source: PomParseError,
    },
}

/// Implementação real de `PomProvider` via HTTP (layout de repositório
/// Maven — seção 6.2 passos 3/4). Permanece **síncrona** de propósito
/// (`reqwest::blocking`, não `tokio`): o fetch de POM acontece um de cada
/// vez durante o BFS sequencial de `graph::build_graph`/`bom::resolve_boms`,
/// sem concorrência real a ganhar — CONVENTIONS.md: "não colocar async por
/// hábito". Isso contrasta com `crate::download::DownloadClient`, que é
/// `async` porque o download de artefatos (seção 6.2 passo 6) é
/// deliberadamente paralelo.
pub struct HttpPomProvider {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl HttpPomProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn pom_url(&self, coordinate: &str, version: &str) -> Result<String, HttpPomError> {
        let (group_id, artifact_id) = coordinate
            .split_once(':')
            .filter(|(g, a)| !g.is_empty() && !a.is_empty())
            .ok_or_else(|| HttpPomError::InvalidCoordinate(coordinate.to_string()))?;

        Ok(format!(
            "{}/{}/{artifact_id}/{version}/{artifact_id}-{version}.pom",
            self.base_url.trim_end_matches('/'),
            group_id.replace('.', "/"),
        ))
    }
}

impl PomProvider for HttpPomProvider {
    fn fetch(
        &self,
        coordinate: &str,
        version: &str,
    ) -> Result<ParsedPom, Box<dyn std::error::Error + Send + Sync>> {
        let url = self.pom_url(coordinate, version)?;

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|source| HttpPomError::Request {
                url: url.clone(),
                source,
            })?;

        if !response.status().is_success() {
            return Err(Box::new(HttpPomError::Status {
                url: url.clone(),
                status: response.status().as_u16(),
            }));
        }

        let xml = response.text().map_err(|source| HttpPomError::Request {
            url: url.clone(),
            source,
        })?;

        parse_pom_xml(&xml)
            .map_err(|source| Box::new(HttpPomError::Parse { url, source }) as Box<_>)
    }
}
