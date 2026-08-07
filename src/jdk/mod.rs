mod adoptium;
mod error;
mod install;
mod list;
mod platform;

pub use adoptium::{AdoptiumClient, JdkRelease};
pub use error::JdkError;
pub use install::{install, is_installed, jdk_install_dir};
pub use list::{find_installed, list_installed};
pub use platform::current_platform;

/// `jvmfast jdk install <version>` só aceita major version por ora
/// (`"21"`, não `"21.0.2-tem"`) — resolver uma versão exata exigiria o
/// endpoint `/v3/assets/version/{version}` do Adoptium, não implementado
/// ainda; rejeitado como erro tipado, nunca truncado silenciosamente para
/// a major version.
pub fn parse_major_version(spec: &str) -> Result<&str, JdkError> {
    if !spec.is_empty() && spec.chars().all(|c| c.is_ascii_digit()) {
        Ok(spec)
    } else {
        Err(JdkError::ExactVersionNotSupported(spec.to_string()))
    }
}

/// Resolve `[project].java-version` (seção 3) para uma major version
/// concreta — `"21"` passa direto por `parse_major_version`; o alias
/// `"lts"` consulta a Adoptium para saber qual é a LTS mais recente *agora*.
/// Só chamado quando o lock está de fato sendo (re)gerado — uma vez
/// resolvido, o valor concreto é persistido em `Lockfile.java_version`
/// (seção 4) e reaproveitado sem nova consulta enquanto o lock for válido.
pub async fn resolve_feature_version(
    spec: &str,
    adoptium: &AdoptiumClient,
) -> Result<String, JdkError> {
    if spec == "lts" {
        Ok(adoptium.most_recent_lts().await?.to_string())
    } else {
        parse_major_version(spec).map(str::to_string)
    }
}
