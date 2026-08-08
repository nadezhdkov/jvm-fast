use super::error::VersionParseError;

/// Versão no formato `major.minor.patch[-prerelease]` — o subconjunto de
/// semver que docs/architecture.md seção 6.1 usa nos exemplos de range.
///
/// **[BLOQUEADOR — ver docs/architecture.md seção 16.2]** Este parser é
/// adequado para interpretar a sintaxe de autoria `^`/`~` do `project.toml`
/// (uma escolha do jvm-fast) e **inadequado para ordenar versões vindas de
/// um repositório**, que é como ele acabou sendo usado por
/// `mediation::compare_versions` e `graph::resolve_version_range`. Versões
/// Maven não são semver, e os contraexemplos não são exóticos:
/// `31.1-jre`/`33.0.0-jre` (a linha estável inteira do Guava — `-jre` é
/// qualificador de plataforma-alvo, não pré-release, e `31.1` nem tem três
/// componentes), `5.3.30.RELEASE`, `1.0`, `9999.0-empty-to-avoid-conflict-with-guava`.
///
/// A consequência mais grave é silenciosa: quando `parse` falha,
/// `mediation::compare_versions` cai em `str::cmp`, onde `"10.0" < "9.0"` —
/// então o critério "versão maior vence" da seção 6.2 seleciona a menor, de
/// forma determinística e sem aviso. A correção é implementar a ordenação
/// `ComparableVersion` do próprio Maven num módulo separado e restringir
/// `SemVer` ao parsing de `^`/`~`; nenhum caminho de ordenação pode manter
/// fallback para `str::cmp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
}

impl SemVer {
    pub fn parse(raw: &str) -> Result<Self, VersionParseError> {
        let (core, pre_release) = match raw.split_once('-') {
            Some((core, pre)) => (core, Some(pre.to_string())),
            None => (raw, None),
        };

        let mut parts = core.split('.');
        let (major, minor, patch) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(major), Some(minor), Some(patch), None) => (major, minor, patch),
            _ => return Err(VersionParseError::InvalidVersion(raw.to_string())),
        };

        let parse_component = |s: &str| {
            s.parse::<u64>()
                .map_err(|_| VersionParseError::InvalidVersion(raw.to_string()))
        };

        Ok(SemVer {
            major: parse_component(major)?,
            minor: parse_component(minor)?,
            patch: parse_component(patch)?,
            pre_release,
        })
    }

    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    fn core(&self) -> (u64, u64, u64) {
        (self.major, self.minor, self.patch)
    }
}

/// Compara só `major.minor.patch`, ignorando pré-release — usado para os
/// limites de range (seção 6.1), que são sempre expressos em termos do
/// núcleo numérico da versão.
pub(super) fn core_ge(a: &SemVer, b: (u64, u64, u64)) -> bool {
    a.core() >= b
}

pub(super) fn core_lt(a: &SemVer, b: (u64, u64, u64)) -> bool {
    a.core() < b
}
