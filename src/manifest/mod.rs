pub mod convert;
pub mod dto;
pub mod error;

pub use error::ManifestError;

use crate::domain::module::Module;
use std::path::Path;

pub fn parse_module(path: &Path) -> Result<Module, ManifestError> {
    let contents = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let manifest: dto::ProjectManifest =
        toml::from_str(&contents).map_err(|e| ManifestError::Toml {
            path: path.to_path_buf(),
            source: e,
        })?;
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    convert::to_module(manifest, root)
}
