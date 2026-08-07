use crate::cli::context::{jdks_root, ADOPTIUM_API};
use crate::cli::CliError;
use crate::config::{config_path, load_defaults, write_default_java_version};
use crate::jdk::{
    current_platform, install, list_installed, parse_major_version, resolve_feature_version,
    AdoptiumClient,
};
use std::io::Write;
use std::path::Path;

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
/// Marca qual delas é o default global (`[defaults].java-version`,
/// `jvmfast jdk use`), se algum estiver configurado.
pub fn list() -> Result<String, CliError> {
    let installed = list_installed(&jdks_root())?;
    if installed.is_empty() {
        return Ok("no JDKs installed — run `jvmfast jdk install <version>`".to_string());
    }

    let default_major = load_defaults(&config_path())?.java_version;
    let lines: Vec<String> = installed
        .into_iter()
        .map(|version| {
            let major = version.split('.').next().unwrap_or(&version);
            if default_major.as_deref() == Some(major) {
                format!("{version} (default)")
            } else {
                version
            }
        })
        .collect();
    Ok(lines.join("\n"))
}

/// `jvmfast jdk use <version>` (seção 7) — define o default global,
/// gravado em `[defaults].java-version` de `~/.config/jvmfast/config.toml`.
/// Exige que a major version já esteja instalada (`jvmfast jdk install`
/// primeiro) — apontar o default para algo que não existe seria um estado
/// inconsistente silencioso, o tipo de coisa que este projeto evita.
pub fn use_jdk(version_spec: &str) -> Result<String, CliError> {
    let feature_version = parse_major_version(version_spec)?;

    let installed = list_installed(&jdks_root())?;
    let is_installed = installed
        .iter()
        .any(|version| version.split('.').next() == Some(feature_version));
    if !is_installed {
        return Err(CliError::JavaVersionNotInstalled(
            feature_version.to_string(),
        ));
    }

    write_default_java_version(&config_path(), feature_version)?;
    Ok(format!("default JDK set to {feature_version}"))
}

/// Resolve `[project].java-version` do manifesto (seção 3: major version
/// simples, ex. `"21"`, ou o alias `"lts"`) para uma major version
/// concreta, e garante que essa JDK está instalada — chamado por
/// `cli::install` só quando o lock está de fato sendo (re)gerado, já que o
/// valor concreto resultante é o que entra em `Lockfile.java_version`.
pub async fn resolve_project_java_version(root: &Path, yes: bool) -> Result<String, CliError> {
    let spec = crate::manifest::parse_java_version(&root.join("project.toml"))?;
    let adoptium = AdoptiumClient::new(ADOPTIUM_API);
    let feature_version = resolve_feature_version(&spec, &adoptium).await?;
    ensure_installed(&feature_version, &adoptium, yes).await?;
    Ok(feature_version)
}

/// Garante que a JDK já persistida em `Lockfile.java_version` (lock
/// reaproveitado, sem re-resolução) está instalada — seção 7: "resolução de
/// java-version... usa a JDK do projeto se instalada, senão dispara
/// instalação automática com confirmação".
pub async fn ensure_project_jdk(feature_version: &str, yes: bool) -> Result<(), CliError> {
    let adoptium = AdoptiumClient::new(ADOPTIUM_API);
    ensure_installed(feature_version, &adoptium, yes).await
}

async fn ensure_installed(
    feature_version: &str,
    adoptium: &AdoptiumClient,
    yes: bool,
) -> Result<(), CliError> {
    let installed = list_installed(&jdks_root())?;
    let already_installed = installed
        .iter()
        .any(|version| version.split('.').next() == Some(feature_version));
    if already_installed {
        return Ok(());
    }

    if !yes && !confirm_install(feature_version) {
        return Err(CliError::JdkInstallDeclined(feature_version.to_string()));
    }

    let (os, arch) = current_platform()?;
    let release = adoptium.latest_release(feature_version, os, arch).await?;
    let client = reqwest::Client::new();
    install(&client, &jdks_root(), &release).await?;
    Ok(())
}

/// Confirmação interativa via stdin (seção 7) — `yes=true` (`--yes`, para
/// CI) pula isso inteiramente em `ensure_installed`, então esta função só
/// roda em uso interativo real. EOF/erro de leitura (ex. stdin não é um
/// terminal) é tratado como "não", nunca como aceitação silenciosa.
fn confirm_install(feature_version: &str) -> bool {
    print!(
        "Java {feature_version} is required by project.toml but is not installed. \
         Install Temurin {feature_version} now? [y/N] "
    );
    let _ = std::io::stdout().flush();

    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}
