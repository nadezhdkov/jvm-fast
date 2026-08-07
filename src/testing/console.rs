use super::error::TestError;
use super::filter::{glob_to_regex, TestFilter};
use crate::cache::CacheStore;
use crate::download::DownloadClient;
use crate::maven::{artifact_filename, artifact_url};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

/// JUnit Platform Console Standalone — "dependência interna do jvm-fast,
/// baixada e cacheada automaticamente" (seção 8.1), nunca declarada em
/// `project.toml`. Coordenada/versão fixas no binário (não configuráveis
/// nesta passada); usa o mesmo cache content-addressable de qualquer outro
/// artefato (seção 5), então uma vez baixada é reaproveitada por todo
/// projeto na máquina, não só o atual. Sempre buscado do Maven Central
/// (`cli::context::MAVEN_CENTRAL`, passado pelo chamador como
/// `base_url`), nunca do `[repositories].default` do projeto — é uma
/// ferramenta do jvm-fast, não uma dependência do usuário que possa estar
/// atrás de um espelho privado.
pub const CONSOLE_COORDINATE: &str = "org.junit.platform:junit-platform-console-standalone";
pub const CONSOLE_VERSION: &str = "1.14.4";

pub async fn ensure_console_jar(
    download_client: &DownloadClient,
    cache_store: &CacheStore,
    base_url: &str,
) -> Result<PathBuf, TestError> {
    let jar_url = artifact_url(base_url, CONSOLE_COORDINATE, CONSOLE_VERSION, "jar")?;
    let filename = artifact_filename(CONSOLE_COORDINATE, CONSOLE_VERSION, "jar")?;
    let resolved = download_client
        .fetch_verify_and_cache(&jar_url, &filename, cache_store)
        .await?;
    Ok(resolved.path)
}

/// Monta e roda `java -jar <console.jar> execute ...` (seção 8.1) — stdio
/// herdado, mesma disciplina de `run::run_main_class`: a saída real do
/// JUnit Console Launcher (relatório em árvore, sumário) vai direto pro
/// terminal do usuário, `jvmfast test` não reformata nada. O exit code do
/// processo já é o sinal de sucesso/falha (0 = tudo passou, seção 8.1 —
/// verificado manualmente contra o binário real, não assumido às cegas
/// como o resto do fluxo Adoptium/Maven deste projeto costuma precisar
/// ser).
#[allow(clippy::too_many_arguments)]
pub fn run(
    java: &Path,
    console_jar: &Path,
    classpath: &[PathBuf],
    scan_dirs: &[PathBuf],
    filter: Option<&TestFilter>,
    reports_dir: Option<&Path>,
) -> Result<ExitStatus, TestError> {
    let classpath_value = std::env::join_paths(classpath).map_err(TestError::Classpath)?;

    let mut command = std::process::Command::new(java);
    command
        .arg("-jar")
        .arg(console_jar)
        .arg("execute")
        .arg("--disable-banner")
        .arg("--classpath")
        .arg(classpath_value);

    for dir in scan_dirs {
        command.arg("--scan-classpath").arg(dir);
    }

    match filter {
        Some(TestFilter::ClassNameGlob(glob)) => {
            command.arg("--include-classname").arg(glob_to_regex(glob));
        }
        Some(TestFilter::Tag(tag)) => {
            command.arg("--include-tag").arg(tag);
        }
        None => {}
    }

    if let Some(dir) = reports_dir {
        command.arg("--reports-dir").arg(dir);
    }

    command.status().map_err(|source| TestError::Spawn {
        path: java.to_path_buf(),
        source,
    })
}
