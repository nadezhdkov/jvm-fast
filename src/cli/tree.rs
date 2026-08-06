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
    let mut output = String::new();
    for module in modules {
        output.push_str(&module.name);
        output.push('\n');
        if let Some(&root_id) = module_roots.get(&module.name) {
            let mut path = vec![root_id];
            write_children(&mut output, graph, root_id, "", &mut path);
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
) {
    let children: Vec<NodeId> = graph
        .edges
        .iter()
        .filter(|e| e.from == from)
        .map(|e| e.to)
        .collect();

    for (i, &child_id) in children.iter().enumerate() {
        let Some(node) = graph.nodes.get(&child_id) else {
            continue;
        };
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&node.coordinate);
        output.push(':');
        output.push_str(&node.selected);

        if path.contains(&child_id) {
            output.push_str(" (cycle)\n");
            continue;
        }
        output.push('\n');

        let next_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
        path.push(child_id);
        write_children(output, graph, child_id, &next_prefix, path);
        path.pop();
    }
}
