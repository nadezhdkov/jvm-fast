use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("could not read manifest at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid TOML in manifest at {path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("invalid dependency coordinate `{0}` — expected `groupId:artifactId`")]
    InvalidCoordinate(String),
}
