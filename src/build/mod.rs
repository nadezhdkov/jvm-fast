mod classpath;
mod compile;
mod error;
mod resources;
mod sources;

pub use classpath::locked_classpath;
pub use error::BuildError;

use crate::cache::CacheStore;
use crate::domain::Workspace;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ModuleBuildSummary {
    pub module: String,
    pub compiled_files: usize,
    pub copied_resources: usize,
    pub classes_dir: PathBuf,
}

/// `jvmfast build` (seção 8): compila `src/main/java` com `javac` e copia
/// `src/main/resources` para `target/classes`, por módulo — itera
/// `workspace.modules` (nunca indexa `[0]`), já que a Fase 5 (multi-módulo)
/// deve funcionar sem reescrita deste core (ver CLAUDE.md, "checklist de
/// compatibilidade com Fase 5"). O classpath vem inteiramente do
/// `project.lock` já resolvido (`classpath::locked_classpath`) — `build`
/// nunca re-resolve nem toca rede.
pub fn build(
    workspace: &Workspace,
    javac: &Path,
    cache_root: &Path,
) -> Result<Vec<ModuleBuildSummary>, BuildError> {
    let cache_store = CacheStore::new(cache_root);
    let dependency_classpath = classpath::locked_classpath(&workspace.lockfile, &cache_store)?;

    let mut summaries = Vec::with_capacity(workspace.modules.len());
    for module in &workspace.modules {
        let module_sources = sources::collect_java_sources(&module.root.join("src/main/java"))?;
        let classes_dir = module.root.join("target/classes");

        compile::compile(javac, &module_sources, &dependency_classpath, &classes_dir)?;
        let copied_resources =
            resources::copy_resources(&module.root.join("src/main/resources"), &classes_dir)?;

        summaries.push(ModuleBuildSummary {
            module: module.name.clone(),
            compiled_files: module_sources.len(),
            copied_resources,
            classes_dir,
        });
    }

    Ok(summaries)
}
