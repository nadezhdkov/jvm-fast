/// Resultado de traduzir um range de versão Maven (docs/architecture.md
/// seção 10) para a sintaxe jvm-fast (`^`/`~`/versão exata, seção 6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeTranslation {
    /// Equivalência direta — versão concreta pronta para escrever no
    /// manifesto gerado.
    Direct(String),
    /// Sem equivalência simples — reportado ao usuário, nunca escrito no
    /// manifesto (ver `translate_maven_range`).
    Unresolved,
}

/// Reconhece a sintaxe de range Maven (`[1.0,2.0)`, `[1.5,)`, `(,2.0]`,
/// `[1.0]`...) — não confundir com a sintaxe jvm-fast (`^2.17.0`,
/// `~2.17.0`), que nunca começa com `[`/`(`.
pub fn is_maven_range(raw: &str) -> bool {
    let trimmed = raw.trim();
    let starts = trimmed.starts_with('[') || trimmed.starts_with('(');
    let ends = trimmed.ends_with(']') || trimmed.ends_with(')');
    starts && ends && trimmed.len() >= 2
}

/// Traduz um range já reconhecido por `is_maven_range`. Só `[x]` (um único
/// valor exato entre colchetes fechados, sem vírgula) tem equivalência
/// direta — todo outro formato (limite aberto de um lado ou dos dois,
/// múltiplos segmentos) precisaria da "maior versão satisfazível no
/// momento do import" (seção 10), que por sua vez exige consultar
/// `maven-metadata.xml` do repositório — a mesma infraestrutura que falta
/// para `jvmfast add` sem versão explícita
/// (`CliError::VersionOmittedNotSupported`) e para
/// `GraphError::UnresolvedVersionRange`. Deliberadamente não implementada
/// aqui; retorna `Unresolved` em vez de arriscar gravar uma versão errada.
pub fn translate_maven_range(raw: &str) -> RangeTranslation {
    let trimmed = raw.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') && !trimmed.contains(',') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if !inner.is_empty() {
            return RangeTranslation::Direct(inner.to_string());
        }
    }
    RangeTranslation::Unresolved
}
