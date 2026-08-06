use super::error::VersionParseError;

/// Versão no formato `major.minor.patch[-prerelease]` — o subconjunto de
/// semver que docs/architecture.md seção 6.1 usa nos exemplos de range.
/// Coordenadas Maven que fogem desse formato (ex. `5.10.2.RELEASE`, só
/// `5.10`) não são suportadas por este parser — escopo deliberado desta
/// passada, não uma limitação escondida.
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
