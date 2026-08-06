use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LockfileError {
    #[error("could not read lockfile at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid TOML in lockfile at {path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("could not serialize lockfile for {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: toml::ser::Error,
    },

    #[error("no checksum available for `{0}` — real artifact download isn't implemented yet")]
    MissingChecksum(String),
}
