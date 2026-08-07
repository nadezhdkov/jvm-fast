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
    #[error(
        "duplicate module name `{0}` — module names (root + [workspace].members) must be unique across the workspace"
    )]
    DuplicateModuleName(String),
}
