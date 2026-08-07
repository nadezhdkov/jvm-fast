use super::error::BuildError;
use std::path::{Path, PathBuf};

/// Invoca `javac -d <out_dir> -cp <classpath> <sources...>` (seção 8) — a
/// JDK usada é sempre a já resolvida via `[project].java-version`
/// (`Lockfile.java_version`, seção 7), nunca uma do `PATH`. Nenhum arquivo
/// fonte é um build bem-sucedido sem-op (módulo sem código ainda produz um
/// `target/classes` vazio, mas existente, para que `copy_resources` tenha
/// onde escrever).
pub fn compile(
    javac: &Path,
    sources: &[PathBuf],
    classpath: &[PathBuf],
    out_dir: &Path,
) -> Result<(), BuildError> {
    std::fs::create_dir_all(out_dir).map_err(|source| BuildError::Io {
        path: out_dir.to_path_buf(),
        source,
    })?;

    if sources.is_empty() {
        return Ok(());
    }

    let mut command = std::process::Command::new(javac);
    command.arg("-d").arg(out_dir);

    if !classpath.is_empty() {
        let classpath_value = std::env::join_paths(classpath).map_err(BuildError::Classpath)?;
        command.arg("-cp").arg(classpath_value);
    }

    command.args(sources);

    let output = command.output().map_err(|source| BuildError::Spawn {
        path: javac.to_path_buf(),
        source,
    })?;

    if !output.status.success() {
        return Err(BuildError::CompileFailed {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(())
}
