use crate::domain::module::Module;
use std::collections::{HashMap, HashSet};

/// Agrega as exclusões declaradas por todos os módulos de um workspace
/// (docs/architecture.md seção 3.4) numa única tabela
/// `coordenada-pai → conjunto de coordenadas excluídas`. Em v1
/// (single-module) isso é equivalente às exclusões do único módulo, mas a
/// função já opera sobre `&[Module]` — nunca um `Module` isolado — para não
/// exigir mudança quando a Fase 5 (multi-módulo) existir (regra da seção
/// 6.2: resolução sempre dispara do `Workspace` sobre a lista de módulos).
///
/// **[DECISÃO]** A arquitetura não desambigua se uma exclusão declarada por
/// um módulo deveria valer só para as transitivas que aquele módulo
/// especificamente traz, ou globalmente no grafo do workspace (seção 6.2:
/// "resolução já é global por construção"). Esta função assume a segunda
/// leitura — se qualquer módulo exclui `filho` quando trazido por `pai`,
/// isso vale para o grafo inteiro — por ser a interpretação mais simples e
/// consistente com uma resolução verdadeiramente global. Revisitar se a
/// Fase 5 revelar um caso de uso que precise de escopo por módulo.
pub fn merge_exclusions(modules: &[Module]) -> HashMap<String, HashSet<String>> {
    let mut merged: HashMap<String, HashSet<String>> = HashMap::new();
    for module in modules {
        for (parent_coordinate, excluded) in &module.exclusions {
            merged
                .entry(parent_coordinate.clone())
                .or_default()
                .extend(excluded.iter().cloned());
        }
    }
    merged
}

/// Decide se `candidate_coordinate`, trazido transitivamente por
/// `parent_coordinate`, deve ser excluído do grafo (seção 3.4) — chamado
/// durante a construção do grafo (seção 6.2, passo 4), antes de adicionar a
/// transitiva como candidata, nunca depois da mediação.
///
/// Não há suporte a wildcard (`"*"`) — fora de escopo da v1 (seção 3.4).
/// Uma exclusão `"*"` só bate contra uma coordenada literalmente igual a
/// `"*"`, o que nunca acontece para uma coordenada Maven real; não há
/// nenhuma expansão especial de wildcard implementada aqui.
pub fn is_excluded(
    exclusions: &HashMap<String, HashSet<String>>,
    parent_coordinate: &str,
    candidate_coordinate: &str,
) -> bool {
    exclusions
        .get(parent_coordinate)
        .is_some_and(|excluded| excluded.contains(candidate_coordinate))
}
