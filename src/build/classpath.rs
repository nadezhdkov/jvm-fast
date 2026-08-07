use super::error::BuildError;
use crate::cache::CacheStore;
use crate::domain::Lockfile;
use crate::maven::artifact_filename;
use std::path::PathBuf;

/// Monta o classpath de compilação/execução a partir do `project.lock` já
/// resolvido (seção 4) — nunca re-resolve nem toca rede; cada entrada é o
/// path content-addressable (seção 5) do JAR já baixado por `jvmfast
/// install`. Um pacote presente no lock mas ausente do cache é erro tipado
/// (`MissingArtifact`), não um classpath incompleto silencioso — sinal de
/// que `jvmfast install` precisa rodar de novo antes de compilar.
pub fn locked_classpath(
    lockfile: &Lockfile,
    cache_store: &CacheStore,
) -> Result<Vec<PathBuf>, BuildError> {
    let mut paths = Vec::with_capacity(lockfile.packages.len());
    for package in &lockfile.packages {
        let filename = artifact_filename(&package.name, &package.version, "jar")?;
        let path = cache_store.artifact_path(&package.sha256, &filename);
        if !path.is_file() {
            return Err(BuildError::MissingArtifact(format!(
                "{}@{}",
                package.name, package.version
            )));
        }
        paths.push(path);
    }
    Ok(paths)
}
