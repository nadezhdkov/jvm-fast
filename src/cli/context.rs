use crate::cli::CliError;
use std::path::{Path, PathBuf};

/// Repositório default quando `[repositories]` não declara nada (seção 3
/// mostra `default = "https://repo1.maven.org/maven2"` como o valor comum).
pub const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";

/// API pública do Eclipse Temurin/Adoptium (seção 7) — única fonte de
/// distribuição de JDK que a arquitetura documenta.
pub const ADOPTIUM_API: &str = "https://api.adoptium.net";

/// Resolve a URL-base do repositório a usar (seção 3: `[repositories]`).
///
/// **Escopo desta passada**: `[repositories]` é um mapa nomeado
/// (`default`, `internal`, ...) e a arquitetura documenta múltiplos
/// repositórios "resolvidos em ordem de declaração" — o que implica tentar
/// cada um até achar o artefato. Essa lógica de fallback multi-repositório
/// não existe ainda; esta função usa a chave `"default"` se declarada,
/// senão cai para o Maven Central. Repositórios nomeados adicionais (ex.
/// `internal`) são parseados por `manifest::parse_repositories` mas ainda
/// não alcançam nenhum fluxo de resolução real — gap sinalizado, não um
/// bug escondido.
pub fn resolve_base_url(root: &Path) -> Result<String, CliError> {
    let repositories = crate::manifest::parse_repositories(&root.join("project.toml"))?;
    Ok(repositories
        .get("default")
        .cloned()
        .unwrap_or_else(|| MAVEN_CENTRAL.to_string()))
}

/// Raiz do cache global (seção 5: `~/.cache/jvmfast/`).
///
/// **Escopo desta passada**: só resolve via `$HOME` (Unix). Suporte
/// multiplataforma de verdade (ex. `%LOCALAPPDATA%` no Windows) ficaria
/// atrás de uma crate como `dirs` — não adicionada ainda porque nada neste
/// projeto até agora precisou rodar fora de Linux/macOS.
pub fn cache_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/jvmfast")
}

/// Raiz das JDKs instaladas (seção 5: `~/.cache/jvmfast/jdks/`).
pub fn jdks_root() -> PathBuf {
    cache_root().join("jdks")
}

/// Resolve `--module <nome>` (seção 12, Fase 5: `jvmfast run`/`jvmfast
/// test` precisam escolher *qual* módulo executar/testar quando o
/// workspace tem mais de um) contra o `Workspace` já carregado. `None`
/// (a flag omitida) sempre cai no módulo raiz — `workspace.modules[0]` por
/// construção de `workspace::load_workspace` (o manifesto raiz é sempre o
/// primeiro lido, antes de qualquer `[workspace].members`) — já que todo
/// workspace tem um módulo raiz real (diferente de um workspace virtual
/// estilo Cargo sem `[project]` próprio); isso preserva o comportamento de
/// antes da Fase 5 sem mudança nenhuma quando não há `[workspace]`
/// nenhum, já que nesse caso o único módulo *é* a raiz.
pub fn resolve_target_module<'a>(
    workspace: &'a crate::domain::Workspace,
    module: Option<&str>,
) -> Result<&'a crate::domain::Module, CliError> {
    match module {
        Some(name) => workspace
            .modules
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| CliError::ModuleNotFound(name.to_string())),
        None => Ok(&workspace.modules[0]),
    }
}
