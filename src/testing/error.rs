use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestError {
    #[error("could not access `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    MavenLayout(#[from] crate::maven::MavenLayoutError),

    #[error(transparent)]
    Download(#[from] crate::download::DownloadError),

    #[error(transparent)]
    Resolve(#[from] crate::resolve::ResolveError),

    #[error(transparent)]
    Build(#[from] crate::build::BuildError),

    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("could not run `{path}`: {source}")]
    Spawn {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not join classpath entries: {0}")]
    Classpath(#[source] std::env::JoinPathsError),
}
