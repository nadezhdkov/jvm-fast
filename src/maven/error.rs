use thiserror::Error;

#[derive(Debug, Error)]
pub enum MavenLayoutError {
    #[error("invalid coordinate `{0}` — expected `groupId:artifactId`")]
    InvalidCoordinate(String),
}
