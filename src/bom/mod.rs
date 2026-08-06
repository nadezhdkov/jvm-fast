mod error;

pub use error::BomResolutionError;

use crate::domain::module::BomReference;
use std::collections::HashMap;

/// Profundidade máxima de import transitivo de BOMs (docs/architecture.md
/// seção 3.3) — evita loops e complexidade de debug. A raiz declarada em
/// `[boms]` conta como profundidade 0; cada import transitivo soma 1.
pub const MAX_IMPORT_DEPTH: u32 = 10;

/// Uma entrada de `<dependencyManagement>` já interpretada. Este módulo não
/// faz parsing de XML — isso é responsabilidade do marco de fetch de POM
/// (`quick-xml`, ainda não implementado); aqui a lógica de resolução de
/// BOM opera sobre um modelo já parseado, fornecido por quem implementar
/// `PomProvider`.
#[derive(Clone)]
pub struct ManagedDependencyEntry {
    pub coordinate: String,
    pub version: String,
    /// `true` para uma entrada `<type>pom</type><scope>import</scope>` —
    /// import transitivo de outro BOM, não uma versão gerenciada direta.
    pub is_bom_import: bool,
}

#[derive(Clone)]
pub struct ParsedPom {
    pub managed_dependencies: Vec<ManagedDependencyEntry>,
}

/// Abstrai de onde um POM vem — fixture local em teste hoje, HTTP + cache
/// (seção 5/6.2) quando o marco de download existir.
pub trait PomProvider {
    fn fetch(
        &self,
        coordinate: &str,
        version: &str,
    ) -> Result<ParsedPom, Box<dyn std::error::Error + Send + Sync>>;
}

/// Constrói a tabela `coordenada → versão` a partir dos BOMs declarados em
/// `[boms]` (seção 3.3) — chamado antes da resolução do grafo de
/// dependências (seção 6.2, passo 3). Regras de precedência:
/// - entre BOMs raiz, o **primeiro listado** em `root_boms` vence;
/// - dentro de um mesmo BOM (incluindo o que vem de imports transitivos),
///   a **primeira entrada declarada** vence.
///
/// Esta função só produz a tabela de versões — nenhuma entrada de BOM entra
/// no grafo de dependências por si só (seção 3.3), porque este módulo não
/// tem nenhuma noção de grafo ou classpath para vazar isso.
pub fn resolve_boms<P: PomProvider>(
    root_boms: &[BomReference],
    provider: &P,
) -> Result<HashMap<String, String>, BomResolutionError> {
    let mut table = HashMap::new();
    for bom_ref in root_boms {
        let pom = fetch_pom(provider, &bom_ref.coordinate, &bom_ref.version)?;
        let bom_table = resolve_pom(&pom, provider, 0)?;
        for (coordinate, version) in bom_table {
            table.entry(coordinate).or_insert(version);
        }
    }
    Ok(table)
}

fn resolve_pom<P: PomProvider>(
    pom: &ParsedPom,
    provider: &P,
    depth: u32,
) -> Result<HashMap<String, String>, BomResolutionError> {
    let mut table = HashMap::new();
    for entry in &pom.managed_dependencies {
        if entry.is_bom_import {
            if depth >= MAX_IMPORT_DEPTH {
                return Err(BomResolutionError::ImportDepthExceeded {
                    coordinate: entry.coordinate.clone(),
                    max: MAX_IMPORT_DEPTH,
                });
            }
            let imported = fetch_pom(provider, &entry.coordinate, &entry.version)?;
            let imported_table = resolve_pom(&imported, provider, depth + 1)?;
            for (coordinate, version) in imported_table {
                table.entry(coordinate).or_insert(version);
            }
        } else {
            table
                .entry(entry.coordinate.clone())
                .or_insert(entry.version.clone());
        }
    }
    Ok(table)
}

fn fetch_pom<P: PomProvider>(
    provider: &P,
    coordinate: &str,
    version: &str,
) -> Result<ParsedPom, BomResolutionError> {
    provider
        .fetch(coordinate, version)
        .map_err(|source| BomResolutionError::Fetch {
            coordinate: coordinate.to_string(),
            version: version.to_string(),
            source,
        })
}
