use std::collections::HashMap;
use std::path::PathBuf;

/// Declara o que um módulo precisa (`project.toml`) — nunca guarda estado
/// resolvido. Ver docs/architecture.md seção 3.1.
pub struct Module {
    pub name: String,
    pub root: PathBuf,
    pub declared_dependencies: Vec<Dependency>,
    pub boms: Vec<BomReference>,
    pub exclusions: HashMap<String, Vec<String>>,
}

pub struct Dependency {
    pub coordinate: String,
    pub version_req: VersionReq,
}

pub enum VersionReq {
    Explicit(String),
    BomManaged,
}

pub struct BomReference {
    pub coordinate: String,
    pub version: String,
}
