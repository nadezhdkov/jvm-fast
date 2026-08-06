use thiserror::Error;

#[derive(Debug, Error)]
pub enum BomResolutionError {
    #[error("BOM import depth exceeded (max {max}) while resolving `{coordinate}`")]
    ImportDepthExceeded { coordinate: String, max: u32 },

    #[error("could not fetch BOM `{coordinate}:{version}`")]
    Fetch {
        coordinate: String,
        version: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
