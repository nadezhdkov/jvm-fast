use crate::cli::context::{cache_root, jdks_root};
use crate::cli::CliError;
use crate::jdk::find_installed;
use crate::lockfile::is_lockfile_valid;
use crate::workspace::{current_manifest_hash, load_workspace};
use std::path::Path;

/// `jvmfast build` (seção 8) — compila `src/main/java` com a JDK do
/// projeto e copia `src/main/resources` para `target/classes`. Nunca
/// resolve nem baixa nada: exige um `project.lock` presente e válido
/// (mesma checagem de `manifest-hash` que `install` usa para decidir se
/// reaproveita o lock, seção 6.2 passo 2) e a JDK de
/// `Lockfile.java_version` já instalada — ambos rejeitados como erro
/// tipado, guiando para `jvmfast install`/`jdk install`, nunca resolvendo
/// nem instalando nada implicitamente por trás de `build`.
pub fn run(root: &Path) -> Result<String, CliError> {
    let workspace = load_workspace(root)?;

    if !root.join("project.lock").is_file() {
        return Err(CliError::LockfileMissing);
    }
    let current_hash = current_manifest_hash(root)?;
    if !is_lockfile_valid(&workspace.lockfile, &current_hash) {
        return Err(CliError::LockfileStale);
    }

    let installed =
        find_installed(&jdks_root(), &workspace.lockfile.java_version)?.ok_or_else(|| {
            CliError::JavaVersionNotInstalled(workspace.lockfile.java_version.clone())
        })?;
    let javac = jdks_root().join(installed).join("bin/javac");

    let summaries = crate::build::build(&workspace, &javac, &cache_root())?;

    let total_compiled: usize = summaries.iter().map(|s| s.compiled_files).sum();
    let total_resources: usize = summaries.iter().map(|s| s.copied_resources).sum();
    Ok(format!(
        "compiled {total_compiled} source file(s), copied {total_resources} resource(s) — {} module(s) built",
        summaries.len()
    ))
}
