use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VersionParseError {
    #[error("invalid version `{0}` — expected `major.minor.patch[-prerelease]`")]
    InvalidVersion(String),
}
