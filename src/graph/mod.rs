mod error;

pub use error::GraphError;

use crate::domain::{Dependency, EdgeKind, GraphEdge, Module, NodeId, VersionReq, VersionRequest};
use crate::exclusion::is_excluded;
use crate::pom::{ParsedPom, PomProvider};
use std::collections::{HashMap, HashSet, VecDeque};

/// Um artefato ainda não mediado — todas as versões pedidas por qualquer
/// módulo/transitiva do workspace, sem decisão de qual venceu ainda (isso é
/// o marco de mediação, seção 6.2 passo 5, ainda não implementado). Não é
/// `ResolvedNode` (docs/architecture.md seção 3.1) de propósito: aquela
/// struct exige `selected`/`mediation_reason`, que não existem antes da
/// mediação — inventar um valor só para preencher o campo seria o tipo de
/// estado falso que este projeto evita (ver `Workspace`/`Lockfile` no
/// bootstrap, seção 3.1 do CLAUDE.md).
pub struct CandidateNode {
    pub coordinate: String,
    pub requests: Vec<VersionRequest>,
}

/// Grafo de candidatos antes da mediação — `edges` já é o `GraphEdge` final
/// da arquitetura (topologia pura, seção 3.1), só `candidates` difere de
/// `DependencyGraph.nodes` por ainda não ter vencedor decidido.
///
/// **[DECISÃO]** `GraphEdge.from` de uma aresta `ModuleDeclared` não tem
/// para onde apontar na arquitetura original — não existe um "nó de módulo"
/// entre os tipos da seção 3.1. `module_roots` resolve isso com um `NodeId`
/// sintético por módulo, que nunca aparece em `candidates` (não representa
/// um artefato resolvido) — só serve de raiz para as arestas diretas.
pub struct CandidateGraph {
    pub candidates: HashMap<NodeId, CandidateNode>,
    pub edges: Vec<GraphEdge>,
    pub module_roots: HashMap<String, NodeId>,
}

struct QueueItem {
    from: NodeId,
    parent_coordinate: Option<String>,
    coordinate: String,
    version: String,
    depth: u32,
    kind: EdgeKind,
    origin_module: String,
}

/// Constrói o grafo de transitivas do workspace inteiro (seção 6.2, passo
/// 4) — opera sempre sobre `&[Module]`, nunca um `Module` isolado, pela
/// mesma regra de compatibilidade com Fase 5 já seguida por `bom` e
/// `exclusion`. Exclusions (seção 3.4) são checadas antes de cada
/// transitiva virar candidata; BOM-managed dependencies (seção 3.3) já
/// devem ter sido resolvidas em `bom_table` antes de chamar esta função.
///
/// **Escopo desta passada**: `VersionReq::Explicit` que ainda é um range
/// (`^`/`~`, seção 6.1) não é resolvido aqui — falta uma fonte de "versões
/// disponíveis" para filtrar contra (metadados de repositório, marco
/// futuro). Um range chega como `GraphError::UnresolvedVersionRange`, nunca
/// tratado silenciosamente como versão literal.
pub fn build_graph<P: PomProvider>(
    modules: &[Module],
    bom_table: &HashMap<String, String>,
    exclusions: &HashMap<String, HashSet<String>>,
    provider: &P,
) -> Result<CandidateGraph, GraphError> {
    let mut next_id: u64 = 0;
    let mut module_roots: HashMap<String, NodeId> = HashMap::new();
    let mut coordinate_ids: HashMap<String, NodeId> = HashMap::new();
    let mut candidates: HashMap<NodeId, CandidateNode> = HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut expanded: HashSet<(String, String)> = HashSet::new();
    let mut queue: VecDeque<QueueItem> = VecDeque::new();

    for module in modules {
        let root_id = NodeId(next_id);
        next_id += 1;
        module_roots.insert(module.name.clone(), root_id);

        for dep in &module.declared_dependencies {
            let version = resolve_declared_version(dep, bom_table)?;
            queue.push_back(QueueItem {
                from: root_id,
                parent_coordinate: None,
                coordinate: dep.coordinate.clone(),
                version,
                depth: 1,
                kind: EdgeKind::ModuleDeclared,
                origin_module: module.name.clone(),
            });
        }
    }

    while let Some(item) = queue.pop_front() {
        if let Some(parent_coordinate) = &item.parent_coordinate {
            if is_excluded(exclusions, parent_coordinate, &item.coordinate) {
                continue;
            }
        }

        let to = *coordinate_ids
            .entry(item.coordinate.clone())
            .or_insert_with(|| {
                let id = NodeId(next_id);
                next_id += 1;
                id
            });

        edges.push(GraphEdge {
            from: item.from,
            to,
            kind: item.kind,
        });

        candidates
            .entry(to)
            .or_insert_with(|| CandidateNode {
                coordinate: item.coordinate.clone(),
                requests: Vec::new(),
            })
            .requests
            .push(VersionRequest {
                version: item.version.clone(),
                origin_module: item.origin_module.clone(),
                depth: item.depth,
            });

        if expanded.insert((item.coordinate.clone(), item.version.clone())) {
            let pom = fetch_pom(provider, &item.coordinate, &item.version)?;
            for transitive in pom.dependencies {
                if !propagates_transitively(&transitive.scope) {
                    continue;
                }
                queue.push_back(QueueItem {
                    from: to,
                    parent_coordinate: Some(item.coordinate.clone()),
                    coordinate: transitive.coordinate,
                    version: transitive.version,
                    depth: item.depth + 1,
                    kind: EdgeKind::External,
                    origin_module: item.origin_module.clone(),
                });
            }
        }
    }

    Ok(CandidateGraph {
        candidates,
        edges,
        module_roots,
    })
}

/// Só `compile`/`runtime` (e o default do Maven, string vazia, que
/// significa `compile`) propagam transitivamente — `test`/`provided`/
/// `system` param aqui: quem declara a dependência os usa diretamente, mas
/// eles nunca viram dependência de quem depende dessa dependência (seção
/// 6.2, semântica real do Maven). Antes desta checagem, `build_graph`
/// tratava todo `<dependency>` de um POM buscado como transitiva de
/// verdade, o que vazava, por exemplo, dependências `test`-scoped de
/// terceiros pro grafo do projeto — descoberto testando `jvmfast test`
/// contra o Maven Central real (`org.assertj:assertj-core` como
/// `[dev-dependencies]`).
fn propagates_transitively(scope: &str) -> bool {
    matches!(scope, "" | "compile" | "runtime")
}

fn resolve_declared_version(
    dep: &Dependency,
    bom_table: &HashMap<String, String>,
) -> Result<String, GraphError> {
    match &dep.version_req {
        VersionReq::BomManaged => bom_table
            .get(&dep.coordinate)
            .cloned()
            .ok_or_else(|| GraphError::MissingBomManagedVersion(dep.coordinate.clone())),
        VersionReq::Explicit(v) if v.starts_with('^') || v.starts_with('~') => {
            Err(GraphError::UnresolvedVersionRange {
                coordinate: dep.coordinate.clone(),
                requirement: v.clone(),
            })
        }
        VersionReq::Explicit(v) => Ok(v.clone()),
    }
}

fn fetch_pom<P: PomProvider>(
    provider: &P,
    coordinate: &str,
    version: &str,
) -> Result<ParsedPom, GraphError> {
    provider
        .fetch(coordinate, version)
        .map_err(|source| GraphError::Fetch {
            coordinate: coordinate.to_string(),
            version: version.to_string(),
            source,
        })
}
