use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("could not run `{path}`: {source}")]
    Spawn {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not join classpath entries: {0}")]
    Classpath(#[source] std::env::JoinPathsError),
}
