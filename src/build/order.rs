use super::error::BuildError;
use crate::domain::Module;
use std::collections::{HashMap, VecDeque};

/// Ordena `modules` topologicamente por `Module.workspace_dependencies`
/// (seção 12, Fase 5) — dependências de workspace sempre antes de quem
/// depende delas, para que `build::build` sempre tenha o `target/classes`
/// de uma dependência já compilado quando chega a vez de compilar quem a
/// usa. Kahn's algorithm: processa primeiro os módulos sem dependências
/// pendentes, e a cada módulo processado libera quem dependia só dele.
/// Devolve índices em `modules` (não clones) — `build::build` precisa da
/// referência original a cada `Module` para montar seu classpath.
///
/// Uma referência a um módulo inexistente é
/// `BuildError::UnknownWorkspaceModule` — mesma checagem que
/// `graph::build_graph` já faz para o mesmo caso durante a resolução, mas
/// repetida aqui porque `build` nunca re-resolve nem re-executa o grafo
/// (opera só sobre `workspace.modules`, seção 8). Um ciclo (A depende de B
/// que depende de A) é `BuildError::CyclicModuleDependency` — nunca uma
/// ordem parcial silenciosa que finge sucesso compilando só parte do
/// ciclo.
pub fn module_order(modules: &[Module]) -> Result<Vec<usize>, BuildError> {
    let index_by_name: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.name.as_str(), index))
        .collect();

    let mut in_degree = vec![0usize; modules.len()];
    let mut dependents_of: Vec<Vec<usize>> = vec![Vec::new(); modules.len()];
    for (index, module) in modules.iter().enumerate() {
        for dependency_name in &module.workspace_dependencies {
            let &dependency_index =
                index_by_name.get(dependency_name.as_str()).ok_or_else(|| {
                    BuildError::UnknownWorkspaceModule {
                        module: module.name.clone(),
                        dependency: dependency_name.clone(),
                    }
                })?;
            dependents_of[dependency_index].push(index);
        }
        in_degree[index] = module.workspace_dependencies.len();
    }

    let mut queue: VecDeque<usize> = (0..modules.len())
        .filter(|&index| in_degree[index] == 0)
        .collect();
    let mut order = Vec::with_capacity(modules.len());
    let mut processed = vec![false; modules.len()];

    while let Some(index) = queue.pop_front() {
        order.push(index);
        processed[index] = true;
        for &dependent in &dependents_of[index] {
            in_degree[dependent] -= 1;
            if in_degree[dependent] == 0 {
                queue.push_back(dependent);
            }
        }
    }

    if order.len() != modules.len() {
        let remaining: Vec<String> = (0..modules.len())
            .filter(|&index| !processed[index])
            .map(|index| modules[index].name.clone())
            .collect();
        return Err(BuildError::CyclicModuleDependency(remaining));
    }

    Ok(order)
}
