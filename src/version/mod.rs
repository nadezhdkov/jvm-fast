mod error;
mod semver;

pub use error::VersionParseError;
pub use semver::SemVer;

use semver::{core_ge, core_lt};

/// Um requisito de versão já interpretado (docs/architecture.md seção 6.1)
/// — `^`/`~` não são estratégias de mediação, só definem quais versões são
/// elegíveis para uma requisição, antes da mediação (seção 6.2) escolher
/// entre requisições concorrentes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionRequirement {
    Exact(SemVer),
    Caret(SemVer),
    Tilde(SemVer),
}

impl VersionRequirement {
    pub fn parse(raw: &str) -> Result<Self, VersionParseError> {
        if let Some(rest) = raw.strip_prefix('^') {
            Ok(VersionRequirement::Caret(SemVer::parse(rest)?))
        } else if let Some(rest) = raw.strip_prefix('~') {
            Ok(VersionRequirement::Tilde(SemVer::parse(rest)?))
        } else {
            Ok(VersionRequirement::Exact(SemVer::parse(raw)?))
        }
    }

    /// Regra da seção 6.1: pré-releases nunca são candidatas automaticamente
    /// quando o requisito não pede explicitamente uma pré-release — só um
    /// requisito `Exact` que é ele mesmo uma pré-release pode casar com uma.
    pub fn matches(&self, candidate: &SemVer) -> bool {
        match self {
            VersionRequirement::Exact(v) => candidate == v,
            VersionRequirement::Caret(v) => {
                !candidate.is_pre_release() && in_bounds(candidate, caret_bounds(v))
            }
            VersionRequirement::Tilde(v) => {
                !candidate.is_pre_release() && in_bounds(candidate, tilde_bounds(v))
            }
        }
    }

    /// Filtra `available` para as versões que satisfazem este requisito,
    /// preservando a ordem de entrada.
    pub fn select_candidates<'a>(&self, available: &'a [SemVer]) -> Vec<&'a SemVer> {
        available.iter().filter(|v| self.matches(v)).collect()
    }
}

fn in_bounds(candidate: &SemVer, (lower, upper): ((u64, u64, u64), (u64, u64, u64))) -> bool {
    core_ge(candidate, lower) && core_lt(candidate, upper)
}

/// `^X.Y.Z` — `>= X.Y.Z, < (X+1).0.0` para major `X >= 1`; para major zero,
/// a compatibilidade é restrita ao primeiro componente não-zero (seção 6.1).
fn caret_bounds(v: &SemVer) -> ((u64, u64, u64), (u64, u64, u64)) {
    let lower = (v.major, v.minor, v.patch);
    let upper = if v.major > 0 {
        (v.major + 1, 0, 0)
    } else if v.minor > 0 {
        (v.major, v.minor + 1, 0)
    } else {
        (v.major, v.minor, v.patch + 1)
    };
    (lower, upper)
}

/// `~X.Y.Z` — `>= X.Y.Z, < X.(Y+1).0` (seção 6.1).
fn tilde_bounds(v: &SemVer) -> ((u64, u64, u64), (u64, u64, u64)) {
    let lower = (v.major, v.minor, v.patch);
    let upper = (v.major, v.minor + 1, 0);
    (lower, upper)
}
