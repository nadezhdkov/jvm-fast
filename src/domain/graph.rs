use std::collections::HashMap;

/// Identificador opaco de um nó no grafo de dependências. A arquitetura
/// (docs/architecture.md seção 3.1) não define a forma concreta — newtype
/// sobre `u64` é a escolha mínima que serve como chave de `HashMap`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub u64);

/// Topologia pura do grafo de resolução — quem trouxe o quê. Ver
/// docs/architecture.md seção 3.1.
pub struct DependencyGraph {
    pub nodes: HashMap<NodeId, ResolvedNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    ModuleDeclared,
    External,
    WorkspaceModule,
}

/// Estado de resolução de um artefato no grafo — sem noção de topologia.
pub struct ResolvedNode {
    pub id: NodeId,
    pub coordinate: String,
    pub requests: Vec<VersionRequest>,
    pub selected: String,
    pub mediation_reason: MediationReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionRequest {
    pub version: String,
    pub origin_module: String,
    pub depth: u32,
}

pub enum MediationReason {
    NearestDepthWins { rejected: Vec<String> },
    HigherVersionWins { rejected: Vec<String> },
    DeterministicTieBreak { rejected: Vec<String> },
    SingleRequest,
}
