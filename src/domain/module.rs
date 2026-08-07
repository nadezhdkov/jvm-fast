use std::collections::HashMap;
use std::path::PathBuf;

/// Declara o que um módulo precisa (`project.toml`) — nunca guarda estado
/// resolvido. Ver docs/architecture.md seção 3.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub root: PathBuf,
    pub declared_dependencies: Vec<Dependency>,
    pub boms: Vec<BomReference>,
    pub exclusions: HashMap<String, Vec<String>>,
    /// Names of other modules in the same `Workspace` this module depends
    /// on (`[workspace-dependencies]`, seção 12 Fase 5) — always a module
    /// *name*, never a Maven coordinate, kept in a separate field/table
    /// rather than folded into `declared_dependencies` since there's no
    /// version to request or mediate for a sibling module: it's a
    /// structural edge (`EdgeKind::WorkspaceModule`), not a resolved
    /// artifact. Sorted alphabetically by `manifest::convert::to_module`
    /// for deterministic build/graph ordering.
    pub workspace_dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub coordinate: String,
    pub version_req: VersionReq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReq {
    Explicit(String),
    BomManaged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomReference {
    pub coordinate: String,
    pub version: String,
}
