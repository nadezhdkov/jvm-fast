use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("failed to download `{url}` after {attempts} attempt(s): {source}")]
    Request {
        url: String,
        attempts: u32,
        #[source]
        source: reqwest::Error,
    },

    #[error("unexpected HTTP status {status} fetching `{url}`")]
    Status { url: String, status: u16 },

    #[error("could not build HTTP client: {0}")]
    ClientBuild(#[source] reqwest::Error),

    #[error("checksum sidecar at `{url}` was empty or unreadable")]
    EmptyChecksum { url: String },

    #[error(transparent)]
    Cache(#[from] crate::cache::CacheError),
}
