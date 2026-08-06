use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("dependency `{0}` is BOM-managed but no BOM provides a version for it")]
    MissingBomManagedVersion(String),

    #[error(
        "version requirement `{requirement}` for `{coordinate}` is a range (^/~) — resolving \
         ranges against available versions isn't implemented yet"
    )]
    UnresolvedVersionRange {
        coordinate: String,
        requirement: String,
    },

    #[error("could not fetch POM `{coordinate}:{version}`")]
    Fetch {
        coordinate: String,
        version: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
