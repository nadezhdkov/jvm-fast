use crate::cli::CliError;
use std::path::{Path, PathBuf};

/// Repositório default quando `[repositories]` não declara nada (seção 3
/// mostra `default = "https://repo1.maven.org/maven2"` como o valor comum).
pub const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";

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
