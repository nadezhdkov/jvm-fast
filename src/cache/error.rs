use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("could not access cache directory at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("checksum mismatch for `{filename}`: expected {expected}, got {actual}")]
    ChecksumMismatch {
        filename: String,
        expected: String,
        actual: String,
    },

    #[error("cache index error ({context}): {source}")]
    Index {
        context: String,
        #[source]
        source: rusqlite::Error,
    },
}
