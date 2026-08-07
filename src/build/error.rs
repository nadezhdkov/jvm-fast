use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("could not access `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    MavenLayout(#[from] crate::maven::MavenLayoutError),

    #[error(
        "artifact `{0}` is in project.lock but was never downloaded — run `jvmfast install` first"
    )]
    MissingArtifact(String),

    #[error("could not run `{path}`: {source}")]
    Spawn {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("javac failed to compile:\n{stderr}")]
    CompileFailed { stderr: String },

    #[error("could not join classpath entries: {0}")]
    Classpath(#[source] std::env::JoinPathsError),
}
