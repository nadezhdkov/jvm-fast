mod adoptium;
mod error;
mod install;
mod list;
mod platform;

pub use adoptium::{AdoptiumClient, JdkRelease};
pub use error::JdkError;
pub use install::{install, is_installed, jdk_install_dir};
pub use list::list_installed;
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
