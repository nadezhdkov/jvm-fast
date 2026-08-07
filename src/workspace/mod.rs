mod error;

pub use error::WorkspaceLoadError;

use crate::domain::{Lockfile, Workspace, WorkspaceConfig};
use crate::lockfile::{compute_manifest_hash, read_lockfile};
use crate::manifest::{parse_module, parse_workspace_members};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Carrega o `Workspace` a partir da raiz do projeto (docs/architecture.md
/// seção 3.1, 6.2 passo 1-2, e seção 12 Fase 5). O manifesto raiz é sempre
/// um módulo em si mesmo (`[project]` é obrigatório em todo manifesto); se
/// ele também declarar `[workspace].members`, cada nome listado é lido como
/// um módulo adicional a partir de `<root>/<member>/project.toml` — na
/// ausência de `[workspace]` (o caso comum), `modules` continua tendo
/// exatamente um `Module`, mesmo comportamento de antes da Fase 5.
///
/// Se `project.lock` não existir ainda, o `Lockfile` retornado é vazio
/// (`packages`/`requests` vazios) com o `manifest-hash` já calculado — um
/// hash agregado de *todos* os manifestos do workspace, na mesma ordem
/// (raiz primeiro, depois `members` na ordem declarada) que
/// `current_manifest_hash` usa, para que os dois sejam sempre comparáveis
/// via `lockfile::is_lockfile_valid` (seção 6.2 passo 2) independente de
/// quantos módulos existem.
pub fn load_workspace(root: &Path) -> Result<Workspace, WorkspaceLoadError> {
    let manifest_entries = collect_manifest_entries(root)?;

    let mut modules = Vec::with_capacity(manifest_entries.len());
    let mut seen_names = HashSet::new();
    for (manifest_path, _) in &manifest_entries {
        let module = parse_module(manifest_path)?;
        if !seen_names.insert(module.name.clone()) {
            return Err(WorkspaceLoadError::DuplicateModuleName(module.name));
        }
        modules.push(module);
    }

    let manifest_hash = compute_manifest_hash(
        manifest_entries
            .iter()
            .map(|(_, contents)| contents.as_str()),
    );

    let lockfile_path = root.join("project.lock");
    // `java_version` fica vazio nesse sentinela de "nunca resolvido" — nunca
    // é lido nesse estado, já que quem orquestra a resolução (`crate::cli`)
    // decide se o lock é reaproveitável olhando a existência do arquivo em
    // disco, não este valor fabricado.
    let lockfile = read_lockfile(&lockfile_path)?.unwrap_or_else(|| Lockfile {
        version: 1,
        manifest_hash,
        java_version: String::new(),
        packages: Vec::new(),
        requests: Vec::new(),
    });

    Ok(Workspace {
        root: root.to_path_buf(),
        modules,
        lockfile,
        config: WorkspaceConfig::default(),
    })
}

/// Recalcula o manifest-hash "agora" de todos os manifestos do workspace
/// (raiz + `[workspace].members`, seção 12 Fase 5), para comparar contra
/// `workspace.lockfile.manifest_hash` via `lockfile::is_lockfile_valid`
/// (seção 6.2 passo 2). Separado de `load_workspace` de propósito: quando um
/// `project.lock` já existe em disco, `load_workspace` carrega o hash
/// *gravado nele*, não o hash atual — decidir se os dois batem é
/// responsabilidade de quem orquestra a resolução (`crate::cli`), não do
/// carregamento do workspace em si.
pub fn current_manifest_hash(root: &Path) -> Result<String, WorkspaceLoadError> {
    let manifest_entries = collect_manifest_entries(root)?;
    Ok(compute_manifest_hash(
        manifest_entries
            .iter()
            .map(|(_, contents)| contents.as_str()),
    ))
}

/// Lê o conteúdo bruto de todos os manifestos do workspace, na ordem raiz →
/// `members` (ordem declarada em `[workspace].members`) — a mesma ordem que
/// `load_workspace`/`current_manifest_hash` precisam para que
/// `compute_manifest_hash` (sensível à ordem) produza sempre o mesmo hash
/// para o mesmo conjunto de módulos. Um `member` cujo diretório ou
/// `project.toml` não existe vira `WorkspaceLoadError::Io` naturalmente —
/// nenhuma checagem de existência separada é necessária.
fn collect_manifest_entries(root: &Path) -> Result<Vec<(PathBuf, String)>, WorkspaceLoadError> {
    let root_manifest_path = root.join("project.toml");
    let root_contents = read_manifest(&root_manifest_path)?;
    let members = parse_workspace_members(&root_manifest_path)?;

    let mut entries = Vec::with_capacity(1 + members.len());
    entries.push((root_manifest_path, root_contents));
    for member in members {
        let member_manifest_path = root.join(&member).join("project.toml");
        let contents = read_manifest(&member_manifest_path)?;
        entries.push((member_manifest_path, contents));
    }
    Ok(entries)
}

fn read_manifest(path: &Path) -> Result<String, WorkspaceLoadError> {
    std::fs::read_to_string(path).map_err(|source| WorkspaceLoadError::Io {
        path: path.to_path_buf(),
        source,
    })
}
