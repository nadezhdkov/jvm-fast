use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceLoadError {
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error(transparent)]
    Lockfile(#[from] crate::lockfile::LockfileError),
    #[error("could not read manifest at {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}
