use super::error::JdkError;

/// Mapeia `std::env::consts::{OS,ARCH}` para os parâmetros que a API do
/// Adoptium espera (`os`/`architecture`).
///
/// **Escopo desta passada**: só Linux/macOS em x86_64/aarch64 — Windows
/// fica de fora por ora (`cli::context::cache_root` também é Unix-only
/// hoje, `$HOME`, mesma limitação já documentada em `CLAUDE.md`).
pub fn current_platform() -> Result<(&'static str, &'static str), JdkError> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        other => {
            return Err(JdkError::UnsupportedPlatform {
                os: other.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            })
        }
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => {
            return Err(JdkError::UnsupportedPlatform {
                os: os.to_string(),
                arch: other.to_string(),
            })
        }
    };
    Ok((os, arch))
}
