mod error;

pub use error::ResolveError;

use crate::bom;
use crate::domain::{DependencyGraph, Module, NodeId};
use crate::exclusion;
use crate::graph;
use crate::mediation;
use crate::pom::PomProvider;
use std::collections::HashMap;

/// Resultado de resolver o workspace inteiro: o grafo mediado, mais os
/// `NodeId` sintéticos por módulo que `graph::build_graph` inventa para
/// raízes de dependência direta (seção 3.1 não modela um "nó de módulo" —
/// ver `[DECISÃO]` em `src/graph/mod.rs`) — carregado adiante para uso
/// futuro de `jvmfast why`/`tree`.
pub struct ResolveOutput {
    pub graph: DependencyGraph,
    pub module_roots: HashMap<String, NodeId>,
}

/// Orquestra os passos 3–5 da seção 6.2 (resolução de BOMs → grafo de
/// transitivas com exclusions aplicadas → mediação de conflitos) — a
/// primeira função que encadeia `bom`/`exclusion`/`graph`/`mediation`
/// ponta a ponta; até este marco cada um só era exercitado isoladamente
/// (ver `tests/resolution_end_to_end.rs`, que já cobre grafo+mediação, mas
/// sem BOMs/exclusions reais no meio). Opera sobre `&[Module]`, nunca um
/// `Module` isolado, pela mesma regra de compatibilidade com Fase 5 que
/// `bom`/`exclusion`/`graph` já seguem.
pub fn resolve<P: PomProvider>(
    modules: &[Module],
    provider: &P,
) -> Result<ResolveOutput, ResolveError> {
    let root_boms: Vec<_> = modules.iter().flat_map(|m| m.boms.clone()).collect();
    let bom_table = bom::resolve_boms(&root_boms, provider)?;
    let exclusions = exclusion::merge_exclusions(modules);
    let candidate_graph = graph::build_graph(modules, &bom_table, &exclusions, provider)?;
    let mediation::MediationResult {
        graph,
        module_roots,
    } = mediation::mediate(candidate_graph);

    Ok(ResolveOutput {
        graph,
        module_roots,
    })
}
