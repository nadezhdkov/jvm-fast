mod error;

pub use error::LockfileError;

use crate::domain::{DependencyGraph, LockedPackage, LockedRequest, Lockfile};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Hash agregado de todos os `project.toml` do workspace (seção 6.2, passo
/// 2) — permite decidir se um `project.lock` existente ainda é válido sem
/// reprocessar a resolução inteira. `manifests` deve ser passado em ordem
/// determinística (ex. módulos ordenados por caminho) — hashear na ordem
/// errada produziria hashes diferentes para o mesmo conjunto de manifestos.
pub fn compute_manifest_hash<'a>(manifests: impl IntoIterator<Item = &'a str>) -> String {
    let mut hasher = Sha256::new();
    for content in manifests {
        hasher.update(content.as_bytes());
    }
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

/// Seção 6.2, passo 2: o lock só é reaproveitado se o `manifest-hash`
/// registrado bater com o hash calculado agora dos manifestos atuais.
pub fn is_lockfile_valid(lockfile: &Lockfile, current_manifest_hash: &str) -> bool {
    lockfile.manifest_hash == current_manifest_hash
}

/// Converte o `DependencyGraph` mediado (seção 3.1, marco de mediação) para
/// `Lockfile` (seção 4) — o passo "Gerar project.lock" do fluxo da seção 6.
///
/// **Escopo desta passada**: `checksums` e `resolved_from` precisam ser
/// fornecidos por quem chama, porque não existe download real de artefato
/// ainda (marco futuro, seção 5/6.2 passo 6) — sem baixar o JAR não há como
/// calcular um SHA-256 real, e `PomProvider` (marco do grafo) não reporta de
/// qual repositório um POM veio. Faltando checksum para algum pacote, é erro
/// tipado (`MissingChecksum`), nunca um valor vazio/fabricado.
pub fn build_lockfile(
    graph: &DependencyGraph,
    manifest_hash: String,
    checksums: &HashMap<String, String>,
    resolved_from: &str,
    java_version: &str,
) -> Result<Lockfile, LockfileError> {
    let mut node_ids: Vec<_> = graph.nodes.keys().collect();
    node_ids.sort_by_key(|id| id.0);

    let mut packages = Vec::with_capacity(node_ids.len());
    let mut requests = Vec::new();

    for &id in &node_ids {
        let node = &graph.nodes[id];
        let key = format!("{}@{}", node.coordinate, node.selected);
        let sha256 = checksums
            .get(&key)
            .cloned()
            .ok_or_else(|| LockfileError::MissingChecksum(key.clone()))?;

        let mut dependencies: Vec<String> = graph
            .edges
            .iter()
            .filter(|edge| &edge.from == id)
            .filter_map(|edge| graph.nodes.get(&edge.to))
            .map(|child| format!("{}@{}", child.coordinate, child.selected))
            .collect();
        dependencies.sort();
        dependencies.dedup();

        packages.push(LockedPackage {
            name: node.coordinate.clone(),
            version: node.selected.clone(),
            sha256,
            resolved_from: resolved_from.to_string(),
            dependencies,
        });

        for request in &node.requests {
            requests.push(LockedRequest {
                module: request.origin_module.clone(),
                name: node.coordinate.clone(),
                version: request.version.clone(),
                depth: request.depth,
            });
        }
    }

    Ok(Lockfile {
        version: 1,
        manifest_hash,
        java_version: java_version.to_string(),
        packages,
        requests,
    })
}

/// `None` quando o arquivo não existe ainda — estado legítimo de "nunca
/// resolvido", não um erro (seção 4: lockfile é gerado automaticamente na
/// primeira resolução).
pub fn read_lockfile(path: &Path) -> Result<Option<Lockfile>, LockfileError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(LockfileError::Io {
                path: path.to_path_buf(),
                source: e,
            })
        }
    };

    toml::from_str(&contents)
        .map(Some)
        .map_err(|e| LockfileError::Toml {
            path: path.to_path_buf(),
            source: e,
        })
}

pub fn write_lockfile(path: &Path, lockfile: &Lockfile) -> Result<(), LockfileError> {
    let contents = toml::to_string_pretty(lockfile).map_err(|e| LockfileError::Serialize {
        path: path.to_path_buf(),
        source: e,
    })?;
    std::fs::write(path, contents).map_err(|e| LockfileError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}
