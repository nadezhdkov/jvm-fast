mod error;

pub use error::WorkspaceLoadError;

use crate::domain::{Lockfile, Workspace, WorkspaceConfig};
use crate::lockfile::{compute_manifest_hash, read_lockfile};
use crate::manifest::parse_module;
use std::path::Path;

/// Carrega o `Workspace` a partir da raiz do projeto (docs/architecture.md
/// seção 3.1, 6.2 passo 1-2) — o primeiro construtor real de `Workspace`
/// neste projeto (até este marco, o tipo só existia declarado, seção 3.1 do
/// CLAUDE.md). Em v1, `Workspace` sempre tem exatamente um `Module` — o
/// `project.toml` da raiz — mas isso é só o que a CLI expõe hoje; nada aqui
/// impede `modules` de crescer na Fase 5.
///
/// Se `project.lock` não existir ainda, o `Lockfile` retornado é vazio
/// (`packages`/`requests` vazios) com o `manifest-hash` já calculado para os
/// manifestos atuais — estado honesto de "nunca resolvido", não um valor
/// fabricado (seção 4: o lock é gerado automaticamente na primeira
/// resolução). Esta função não decide se um lock existente ainda é válido
/// — isso é `lockfile::is_lockfile_valid`, chamado por quem orquestra a
/// resolução (marco futuro de comandos CLI).
pub fn load_workspace(root: &Path) -> Result<Workspace, WorkspaceLoadError> {
    let manifest_path = root.join("project.toml");

    let manifest_contents =
        std::fs::read_to_string(&manifest_path).map_err(|source| WorkspaceLoadError::Io {
            path: manifest_path.clone(),
            source,
        })?;
    let module = parse_module(&manifest_path)?;

    let manifest_hash = compute_manifest_hash([manifest_contents.as_str()]);

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
        modules: vec![module],
        lockfile,
        config: WorkspaceConfig::default(),
    })
}

/// Recalcula o manifest-hash "agora" dos manifestos do workspace, para
/// comparar contra `workspace.lockfile.manifest_hash` via
/// `lockfile::is_lockfile_valid` (seção 6.2 passo 2). Separado de
/// `load_workspace` de propósito: quando um `project.lock` já existe em
/// disco, `load_workspace` carrega o hash *gravado nele*, não o hash atual
/// — decidir se os dois batem é responsabilidade de quem orquestra a
/// resolução (`crate::cli`), não do carregamento do workspace em si.
pub fn current_manifest_hash(root: &Path) -> Result<String, WorkspaceLoadError> {
    let manifest_path = root.join("project.toml");
    let manifest_contents =
        std::fs::read_to_string(&manifest_path).map_err(|source| WorkspaceLoadError::Io {
            path: manifest_path,
            source,
        })?;
    Ok(compute_manifest_hash([manifest_contents.as_str()]))
}
