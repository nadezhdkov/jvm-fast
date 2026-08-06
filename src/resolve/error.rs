use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error(transparent)]
    Bom(#[from] crate::bom::BomResolutionError),

    #[error(transparent)]
    Graph(#[from] crate::graph::GraphError),
}
