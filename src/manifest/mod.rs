pub mod convert;
pub mod dto;
pub mod error;

pub use error::ManifestError;

use crate::domain::module::Module;
use std::collections::HashMap;
use std::path::Path;

pub fn parse_module(path: &Path) -> Result<Module, ManifestError> {
    let manifest = parse_manifest(path)?;
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    convert::to_module(manifest, root)
}

/// `[repositories]` (seção 3) não tem campo equivalente em `Module` — por
/// design, seção 3.1 não modela repositórios no domínio (nem `Module` nem
/// `Workspace`). É configuração de *como* resolver, não *o que* resolver,
/// então a camada de orquestração (`crate::cli`) lê isso direto do
/// manifesto, sem passar pelo domínio. Reler o arquivo aqui (em vez de
/// estender `parse_module`) segue o mesmo precedente de
/// `workspace::load_workspace`, que também relê `project.toml` para o hash
/// em vez de remodelar `Module`.
pub fn parse_repositories(path: &Path) -> Result<HashMap<String, String>, ManifestError> {
    Ok(parse_manifest(path)?.repositories)
}

fn parse_manifest(path: &Path) -> Result<dto::ProjectManifest, ManifestError> {
    let contents = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    toml::from_str(&contents).map_err(|e| ManifestError::Toml {
        path: path.to_path_buf(),
        source: e,
    })
}
