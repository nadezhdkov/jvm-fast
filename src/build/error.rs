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

    #[error(
        "module `{module}` declares a [workspace-dependencies] entry on `{dependency}`, but no module named `{dependency}` exists in this workspace"
    )]
    UnknownWorkspaceModule { module: String, dependency: String },

    #[error(
        "cyclic [workspace-dependencies] involving module(s): {0:?} — a module can't (transitively) depend on itself"
    )]
    CyclicModuleDependency(Vec<String>),
}
