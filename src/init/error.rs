use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InitError {
    #[error(
        "{0} already exists — this project is already initialized, run `jvmfast install`/`jvmfast build` directly"
    )]
    ManifestAlreadyExists(PathBuf),

    #[error(
        "{0} exists — this looks like a Maven project already; run `jvmfast import-pom` instead of `jvmfast init` to preserve its dependencies"
    )]
    PomXmlDetected(PathBuf),

    #[error("could not derive a project name from {0} — pass an explicit `--name`")]
    CouldNotDeriveName(PathBuf),

    #[error("could not write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
