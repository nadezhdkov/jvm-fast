use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GradleImportError {
    #[error("could not read/write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "{0} already exists — `jvmfast import-gradle` never overwrites an existing manifest, remove or rename it first"
    )]
    ManifestAlreadyExists(PathBuf),

    #[error(
        "{0} has no gradlew/gradlew.bat — the Gradle Tooling API still needs a real Gradle distribution to connect to (seção 10); jvmfast only avoids needing to understand its version, not needing one to exist at all"
    )]
    GradlewNotFound(PathBuf),

    #[error(transparent)]
    Bridge(#[from] crate::gradlebridge::GradleBridgeError),

    #[error("could not invoke `java` to run jvmfast-gradle-bridge.jar: {0}")]
    JavaNotFound(#[source] std::io::Error),

    #[error("jvmfast-gradle-bridge.jar exited with status {status}: {stderr}")]
    BridgeFailed { status: i32, stderr: String },

    #[error("jvmfast-gradle-bridge.jar produced output that isn't the expected JSON model: {0}")]
    InvalidBridgeOutput(#[from] serde_json::Error),

    #[error("jvmfast-gradle-bridge.jar reported zero Gradle modules — nothing to import")]
    NoModulesInBridgeOutput,
}
