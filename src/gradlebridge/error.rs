use thiserror::Error;

#[derive(Debug, Error)]
pub enum GradleBridgeError {
    #[error("failed to extract embedded jvmfast-gradle-bridge.jar: {0}")]
    Cache(#[source] crate::cache::CacheError),
}
