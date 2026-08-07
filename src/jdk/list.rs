use super::error::JdkError;
use std::path::Path;

/// Lista as JDKs instaladas em `jdks_root` (nomes de diretório, ex.
/// `"21.0.2-tem"`) — `jvmfast jdk list` (seção 7). Diretório ausente é uma
/// lista vazia, não um erro (estado honesto de "nenhuma JDK instalada
/// ainda").
pub fn list_installed(jdks_root: &Path) -> Result<Vec<String>, JdkError> {
    if !jdks_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut names: Vec<String> = std::fs::read_dir(jdks_root)
        .map_err(|source| JdkError::Io {
            path: jdks_root.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}

/// Encontra, entre as JDKs instaladas, a que corresponde à major version
/// pedida (`"21"` casa com o diretório `"21.0.2-tem"`) — usado por `jdk
/// list`/`jdk use`/`ensure_installed` (seção 7) e por `cli::build` para
/// localizar `bin/javac` da JDK do projeto (`Lockfile.java_version`).
pub fn find_installed(jdks_root: &Path, feature_version: &str) -> Result<Option<String>, JdkError> {
    Ok(list_installed(jdks_root)?
        .into_iter()
        .find(|version| version.split('.').next() == Some(feature_version)))
}
