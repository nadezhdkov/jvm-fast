use crate::domain::{DependencyGraph, Module, NodeId};
use std::collections::HashMap;

/// Formata a árvore de dependências resolvida (`jvmfast tree`, seção 9) a
/// partir do `DependencyGraph`/`module_roots` já em memória (produzidos por
/// `crate::resolve::resolve`) — função pura, sem I/O, para ser testável sem
/// rede nem filesystem.
pub fn format_tree(
    graph: &DependencyGraph,
    module_roots: &HashMap<String, NodeId>,
    modules: &[Module],
) -> String {
    // Inverso de `module_roots` (seção 12, Fase 5): uma aresta
    // `EdgeKind::WorkspaceModule` aponta `to` para o `NodeId` sintético de
    // outro módulo, nunca para um `ResolvedNode` real em `graph.nodes` —
    // sem isso, `write_children` simplesmente descartaria essas arestas
    // (mesmo `let Some(...) else { continue }` que já protege contra
    // `NodeId`s desconhecidos), tornando dependências entre módulos
    // invisíveis em `jvmfast tree`.
    let root_names: HashMap<NodeId, &str> = module_roots
        .iter()
        .map(|(name, id)| (*id, name.as_str()))
        .collect();

    let mut output = String::new();
    for module in modules {
        output.push_str(&module.name);
        output.push('\n');
        if let Some(&root_id) = module_roots.get(&module.name) {
            let mut path = vec![root_id];
            write_children(&mut output, graph, root_id, "", &mut path, &root_names);
        }
    }
    output
}

fn write_children(
    output: &mut String,
    graph: &DependencyGraph,
    from: NodeId,
    prefix: &str,
    path: &mut Vec<NodeId>,
    root_names: &HashMap<NodeId, &str>,
) {
    let children: Vec<NodeId> = graph
        .edges
        .iter()
        .filter(|e| e.from == from)
        .map(|e| e.to)
        .collect();

    for (i, &child_id) in children.iter().enumerate() {
        let label = if let Some(node) = graph.nodes.get(&child_id) {
            format!("{}:{}", node.coordinate, node.selected)
        } else if let Some(&name) = root_names.get(&child_id) {
            format!("{name} (workspace module)")
        } else {
            continue;
        };

        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&label);

        if path.contains(&child_id) {
            output.push_str(" (cycle)\n");
            continue;
        }
        output.push('\n');

        let next_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
        path.push(child_id);
        write_children(output, graph, child_id, &next_prefix, path, root_names);
        path.pop();
    }
}
