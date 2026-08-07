use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Parse(#[from] crate::pom::PomParseError),

    #[error(
        "{0} already exists — `jvmfast import-pom` never overwrites an existing manifest, remove or rename it first"
    )]
    ManifestAlreadyExists(PathBuf),

    #[error(
        "pom.xml has no direct <project><artifactId> — parent-POM inheritance (seção 13) is not supported, add one explicitly before importing"
    )]
    MissingArtifactId,

    #[error(
        "pom.xml has no direct <project><version> — parent-POM inheritance (seção 13) is not supported, add one explicitly before importing"
    )]
    MissingVersion,
}
