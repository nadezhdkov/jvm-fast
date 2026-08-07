pub mod convert;
pub mod dto;
pub mod error;

pub use error::ManifestError;

use crate::domain::module::Module;
use std::collections::HashMap;
use std::path::Path;

pub fn parse_module(path: &Path) -> Result<Module, ManifestError> {
    let manifest = parse_manifest(path)?;
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    convert::to_module(manifest, root)
}

/// `[dev-dependencies]` (seção 3, 8.1) como um `Module` sintético — ver
/// `convert::to_dev_module`. Reparseia o manifesto independentemente de
/// `parse_module`, mesmo precedente de `parse_repositories`/
/// `parse_java_version` (releitura barata em vez de compartilhar um parse).
pub fn parse_dev_module(path: &Path) -> Result<Option<Module>, ManifestError> {
    let manifest = parse_manifest(path)?;
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    convert::to_dev_module(manifest, root)
}

/// `[repositories]` (seção 3) não tem campo equivalente em `Module` — por
/// design, seção 3.1 não modela repositórios no domínio (nem `Module` nem
/// `Workspace`). É configuração de *como* resolver, não *o que* resolver,
/// então a camada de orquestração (`crate::cli`) lê isso direto do
/// manifesto, sem passar pelo domínio. Reler o arquivo aqui (em vez de
/// estender `parse_module`) segue o mesmo precedente de
/// `workspace::load_workspace`, que também relê `project.toml` para o hash
/// em vez de remodelar `Module`.
pub fn parse_repositories(path: &Path) -> Result<HashMap<String, String>, ManifestError> {
    Ok(parse_manifest(path)?.repositories)
}

/// `[project].java-version` (seção 3) — mesmo raciocínio de
/// `parse_repositories`: `Module` (seção 3.1) não tem campo para isso, é
/// configuração de qual JDK usar, não intenção de dependências. Devolve a
/// string crua (`"21"`, `"lts"`) sem resolver — isso é `jdk::resolve_*`
/// (seção 7), que sabe interpretar o alias `"lts"`.
pub fn parse_java_version(path: &Path) -> Result<String, ManifestError> {
    Ok(parse_manifest(path)?.project.java_version)
}

/// `[run]` (seção 3) — mesmo raciocínio de `parse_repositories`: é
/// configuração de *como* executar, não algo que `Module` (seção 3.1)
/// modela. `[run]` inteiro é opcional no manifesto (nem todo projeto tem
/// `main-class` definido, ex. bibliotecas sem entry point), então ausência
/// vira `RunConfig` "vazio" honesto, não erro — `jvmfast run` (seção 8) é
/// quem decide que a ausência de `main-class` é um problema pra *ele*.
pub struct RunConfig {
    pub main_class: Option<String>,
    pub jvm_args: Vec<String>,
}

/// `[workspace].members` (seção 12, Fase 5) — nomes de subdiretórios que
/// também são módulos, cada um com seu próprio `project.toml` completo.
/// Ausência de `[workspace]` inteiro (o caso comum, projeto single-module)
/// devolve uma lista vazia, não um erro — nem todo projeto é um workspace.
/// Só o manifesto raiz é lido para isso; `workspace::load_workspace` é quem
/// decide, a partir do resultado, quais outros manifestos carregar.
pub fn parse_workspace_members(path: &Path) -> Result<Vec<String>, ManifestError> {
    Ok(parse_manifest(path)?
        .workspace
        .map(|section| section.members)
        .unwrap_or_default())
}

pub fn parse_run_config(path: &Path) -> Result<RunConfig, ManifestError> {
    let run = parse_manifest(path)?.run;
    Ok(match run {
        Some(run) => RunConfig {
            main_class: run.main_class,
            jvm_args: run.jvm_args,
        },
        None => RunConfig {
            main_class: None,
            jvm_args: Vec::new(),
        },
    })
}

fn parse_manifest(path: &Path) -> Result<dto::ProjectManifest, ManifestError> {
    let contents = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    toml::from_str(&contents).map_err(|e| ManifestError::Toml {
        path: path.to_path_buf(),
        source: e,
    })
}
