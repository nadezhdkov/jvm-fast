use crate::build::locked_classpath;
use crate::cache::CacheStore;
use crate::cli::context::{cache_root, jdks_root};
use crate::cli::CliError;
use crate::jdk::find_installed;
use crate::lockfile::is_lockfile_valid;
use crate::manifest::parse_run_config;
use crate::run::run_main_class;
use crate::workspace::{current_manifest_hash, load_workspace};
use std::path::Path;

/// `jvmfast run` (seção 8): compila (sempre — v1 não tem build incremental,
/// mesmo raciocínio já documentado em `cli::build`) e então executa
/// `[run].main-class` com `[run].jvm-args`, classpath = `target/classes` de
/// cada módulo + dependências do `project.lock`. Mesmas checagens de
/// `cli::build` (lock presente e válido, JDK do projeto instalada) — `run`
/// nunca resolve/baixa/instala nada implicitamente.
pub fn run(root: &Path) -> Result<String, CliError> {
    let workspace = load_workspace(root)?;

    if !root.join("project.lock").is_file() {
        return Err(CliError::LockfileMissing);
    }
    let current_hash = current_manifest_hash(root)?;
    if !is_lockfile_valid(&workspace.lockfile, &current_hash) {
        return Err(CliError::LockfileStale);
    }

    let run_config = parse_run_config(&root.join("project.toml"))?;
    let main_class = run_config
        .main_class
        .ok_or(CliError::MainClassNotConfigured)?;

    let installed =
        find_installed(&jdks_root(), &workspace.lockfile.java_version)?.ok_or_else(|| {
            CliError::JavaVersionNotInstalled(workspace.lockfile.java_version.clone())
        })?;
    let jdk_dir = jdks_root().join(installed);
    let javac = jdk_dir.join("bin/javac");
    let java = jdk_dir.join("bin/java");

    let build_summaries = crate::build::build(&workspace, &javac, &cache_root())?;

    let cache_store = CacheStore::new(cache_root());
    let mut classpath = locked_classpath(&workspace.lockfile, &cache_store)?;
    classpath.extend(
        build_summaries
            .into_iter()
            .map(|summary| summary.classes_dir),
    );

    let status = run_main_class(&java, &classpath, &run_config.jvm_args, &main_class)?;
    if !status.success() {
        // `.code()` é `None` só quando o processo foi terminado por sinal
        // (Unix) — sem código de saída real pra reportar, então -1 marca
        // esse caso sem inventar um número que pareça um exit code válido.
        return Err(CliError::ProgramExited(status.code().unwrap_or(-1)));
    }

    Ok(format!("{main_class} exited successfully"))
}
