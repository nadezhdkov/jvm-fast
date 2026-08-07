use crate::cli::context::{jdks_root, ADOPTIUM_API};
use crate::cli::CliError;
use crate::jdk::{current_platform, install, list_installed, parse_major_version, AdoptiumClient};

/// `jvmfast jdk install <version>` (seção 7) — só major version por ora,
/// ver `jdk::parse_major_version`.
pub async fn install_jdk(version_spec: &str) -> Result<String, CliError> {
    let feature_version = parse_major_version(version_spec)?;
    let (os, arch) = current_platform()?;

    let adoptium = AdoptiumClient::new(ADOPTIUM_API);
    let release = adoptium.latest_release(feature_version, os, arch).await?;

    let client = reqwest::Client::new();
    let root = jdks_root();
    let path = install(&client, &root, &release).await?;

    Ok(format!(
        "Temurin {} installed at {}",
        release.version,
        path.display()
    ))
}

/// `jvmfast jdk list` (seção 7) — só instaladas por ora; listar versões
/// *disponíveis* exigiria enumerar releases do Adoptium por major version,
/// não só a última (fora de escopo desta passada, que só resolve "latest").
pub fn list() -> Result<String, CliError> {
    let installed = list_installed(&jdks_root())?;
    if installed.is_empty() {
        Ok("no JDKs installed — run `jvmfast jdk install <version>`".to_string())
    } else {
        Ok(installed.join("\n"))
    }
}
