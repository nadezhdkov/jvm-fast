use crate::domain::{DependencyGraph, MediationReason, NodeId};
use std::collections::{HashMap, HashSet, VecDeque};

/// Diagnóstico de origem de um artefato (`jvmfast why`, seção 9.1) — função
/// pura sobre um `DependencyGraph`/`module_roots` já resolvidos em memória,
/// sem I/O. `None` se a coordenada não está no grafo resolvido.
///
/// **Escopo desta passada**: a seção 9.1 descreve `why` reconstruindo o
/// diagnóstico só a partir de `project.lock`, sem re-fetch — essa é a meta
/// de longo prazo, mas o `Lockfile` atual (`src/domain/lockfile.rs`) não
/// persiste `mediation_reason`, então uma reconstrução fiel exigiria
/// estender o schema do lock primeiro (gap sinalizado, não implementado
/// aqui). Esta passada resolve o workspace de novo em memória (mesmo
/// caminho de `install`) e formata a partir do resultado — corretude
/// idêntica, só sem a otimização de "não re-fetch" que o doc descreve.
pub fn format_why(
    graph: &DependencyGraph,
    module_roots: &HashMap<String, NodeId>,
    coordinate: &str,
) -> Option<String> {
    let (&target_id, target_node) = graph
        .nodes
        .iter()
        .find(|(_, node)| node.coordinate == coordinate)?;

    let mut output = format!("{coordinate}:{}\n\nRequested by:\n\n", target_node.selected);

    // Inverso de `module_roots` (seção 12, Fase 5) — mesma necessidade de
    // `cli::tree::format_tree`: um passo do caminho pode ser o `NodeId`
    // sintético de outro módulo (uma aresta `EdgeKind::WorkspaceModule`
    // atravessada pela BFS abaixo), não um `ResolvedNode` real.
    let root_names: HashMap<NodeId, &str> = module_roots
        .iter()
        .map(|(name, id)| (*id, name.as_str()))
        .collect();

    let mut module_names: Vec<&String> = module_roots.keys().collect();
    module_names.sort();

    for module_name in module_names {
        let root_id = module_roots[module_name];
        let Some(path) = shortest_path(graph, root_id, target_id) else {
            continue;
        };

        output.push_str(module_name);
        output.push('\n');
        let steps: Vec<&NodeId> = path.iter().skip(1).collect();
        for (i, &step_id) in steps.iter().enumerate() {
            let label = if let Some(node) = graph.nodes.get(step_id) {
                format!("{}:{}", node.coordinate, node.selected)
            } else if let Some(&name) = root_names.get(step_id) {
                format!("{name} (workspace module)")
            } else {
                continue;
            };
            let indent = "    ".repeat(i);
            output.push_str(&indent);
            output.push_str("└── ");
            output.push_str(&label);
            output.push('\n');
        }
        output.push('\n');
    }

    output.push_str("Resolution:\n");
    output.push_str(&format!(
        "  selected: {coordinate}:{}\n",
        target_node.selected
    ));
    output.push_str(&format!(
        "  reason: {}\n",
        format_reason(&target_node.mediation_reason)
    ));

    Some(output)
}

fn shortest_path(graph: &DependencyGraph, from: NodeId, target: NodeId) -> Option<Vec<NodeId>> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(vec![from]);
    visited.insert(from);

    while let Some(path) = queue.pop_front() {
        let &last = path.last().expect("path always has at least one node");
        if last == target {
            return Some(path);
        }
        for edge in graph.edges.iter().filter(|e| e.from == last) {
            if visited.insert(edge.to) {
                let mut next_path = path.clone();
                next_path.push(edge.to);
                queue.push_back(next_path);
            }
        }
    }
    None
}

fn format_reason(reason: &MediationReason) -> String {
    match reason {
        MediationReason::SingleRequest => "only one version was requested".to_string(),
        MediationReason::NearestDepthWins { rejected } => {
            format!("nearest dependency wins over: {}", rejected.join(", "))
        }
        MediationReason::HigherVersionWins { rejected } => format!(
            "same dependency depth; higher version selected as tie-breaker over: {}",
            rejected.join(", ")
        ),
        MediationReason::DeterministicTieBreak { rejected } => format!(
            "same depth and version; deterministic tie-break over: {}",
            rejected.join(", ")
        ),
    }
}
