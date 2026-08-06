use crate::domain::{DependencyGraph, MediationReason, NodeId, ResolvedNode, VersionRequest};
use crate::graph::CandidateGraph;
use crate::version::SemVer;
use std::cmp::Ordering;
use std::collections::HashMap;

pub struct MediationResult {
    pub graph: DependencyGraph,
    /// Repassado do `CandidateGraph` (marco anterior) — `NodeId` sintético
    /// por módulo, raiz das arestas `ModuleDeclared`. `DependencyGraph`
    /// (seção 3.1) não tem campo para isso; quem precisar traduzir um
    /// `NodeId` de volta a um nome de módulo (ex. um futuro `jvmfast why`)
    /// usa este mapa em vez de reconstruí-lo.
    pub module_roots: HashMap<String, NodeId>,
}

/// Aplica a mediação de conflitos (docs/architecture.md seção 6.2, passo 5)
/// sobre um `CandidateGraph` (seção 6.2, passo 4 — marco anterior),
/// produzindo o `DependencyGraph`/`ResolvedNode` reais da arquitetura
/// (seção 3.1) — a primeira vez que esses tipos são efetivamente
/// construídos neste projeto, em vez de só declarados.
///
/// Precedência fixa, nunca heurísticas concorrentes: profundidade menor
/// vence primeiro; só quando dois pedidos empatam em profundidade o
/// critério de versão maior é avaliado; só quando também empatam em versão
/// entra um desempate determinístico. Cada `VersionRequest` é tratado como
/// seu próprio candidato — mesmo dois pedidos com a mesma versão vinda de
/// módulos diferentes disputam via `DeterministicTieBreak` (seção 13.1:
/// "mesma profundidade, mesma versão, módulos diferentes" é uma categoria
/// própria, não tratada como ausência de conflito).
pub fn mediate(candidate_graph: CandidateGraph) -> MediationResult {
    let mut nodes = HashMap::with_capacity(candidate_graph.candidates.len());
    for (id, candidate) in candidate_graph.candidates {
        let (selected, mediation_reason) = decide(&candidate.requests);
        nodes.insert(
            id,
            ResolvedNode {
                id,
                coordinate: candidate.coordinate,
                requests: candidate.requests,
                selected,
                mediation_reason,
            },
        );
    }

    MediationResult {
        graph: DependencyGraph {
            nodes,
            edges: candidate_graph.edges,
        },
        module_roots: candidate_graph.module_roots,
    }
}

fn decide(requests: &[VersionRequest]) -> (String, MediationReason) {
    let mut ranked: Vec<&VersionRequest> = requests.iter().collect();
    ranked.sort_by(|a, b| {
        a.depth
            .cmp(&b.depth)
            .then_with(|| compare_versions(&a.version, &b.version).reverse())
            .then_with(|| a.origin_module.cmp(&b.origin_module))
    });

    if ranked.len() == 1 {
        return (ranked[0].version.clone(), MediationReason::SingleRequest);
    }

    let winner = ranked[0];
    let runner_up = ranked[1];
    let rejected: Vec<String> = ranked[1..].iter().map(|r| r.version.clone()).collect();

    let reason = if winner.depth < runner_up.depth {
        MediationReason::NearestDepthWins { rejected }
    } else if compare_versions(&winner.version, &runner_up.version) != Ordering::Equal {
        MediationReason::HigherVersionWins { rejected }
    } else {
        MediationReason::DeterministicTieBreak { rejected }
    };

    (winner.version.clone(), reason)
}

/// Compara duas versões numericamente via `SemVer` (major.minor.patch,
/// seção 6.1) quando ambas são parseáveis — uma release sem sufixo vence
/// uma pré-release de mesmo núcleo numérico. Cai para comparação de string
/// quando alguma foge desse formato — mesmo corte de escopo documentado em
/// `crate::version::SemVer::parse`; não é "correto" para versões Maven que
/// fogem de major.minor.patch, mas é determinístico.
fn compare_versions(a: &str, b: &str) -> Ordering {
    match (SemVer::parse(a), SemVer::parse(b)) {
        (Ok(va), Ok(vb)) => (va.major, va.minor, va.patch)
            .cmp(&(vb.major, vb.minor, vb.patch))
            .then_with(|| match (&va.pre_release, &vb.pre_release) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(pa), Some(pb)) => pa.cmp(pb),
            }),
        _ => a.cmp(b),
    }
}
