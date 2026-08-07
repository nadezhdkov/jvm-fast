use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JdkError {
    #[error("HTTP request to `{url}` failed: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("unexpected HTTP status {status} fetching `{url}`")]
    Status { url: String, status: u16 },

    #[error("could not parse Adoptium API response from `{url}`: {source}")]
    Parse {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Adoptium API returned no releases for feature version `{0}`")]
    NoReleaseFound(String),

    #[error(
        "unsupported platform for automatic JDK install: os=`{os}`, arch=`{arch}` — install \
         manually and point `java-version`/JAVA_HOME at it"
    )]
    UnsupportedPlatform { os: String, arch: String },

    #[error("checksum mismatch for JDK archive `{filename}`: expected {expected}, got {actual}")]
    ChecksumMismatch {
        filename: String,
        expected: String,
        actual: String,
    },

    #[error("could not extract JDK archive: {0}")]
    Extract(#[source] std::io::Error),

    #[error("could not access `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "`jvmfast jdk install` with an exact version (`{0}`) is not supported yet — only major \
         versions (e.g. `21`) are, since this uses the Adoptium \"latest\" endpoint, which only \
         resolves by feature version"
    )]
    ExactVersionNotSupported(String),
}
