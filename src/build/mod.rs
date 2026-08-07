mod classpath;
mod compile;
mod error;
mod order;
mod resources;
mod sources;

pub use classpath::locked_classpath;
pub use compile::compile;
pub use error::BuildError;
pub use order::module_order;
pub use resources::copy_resources;
pub use sources::collect_java_sources;

use crate::cache::CacheStore;
use crate::domain::Workspace;
use std::collections::HashMap;
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
/// `workspace.modules` em ordem topológica (`order::module_order`, seção
/// 12 Fase 5: dependências de workspace antes de quem depende delas), não
/// na ordem declarada em `[workspace].members`. O classpath externo
/// (`project.lock` já resolvido) é o mesmo para todos os módulos; cada
/// módulo soma a ele o `target/classes` de cada nome em
/// `Module.workspace_dependencies` — já compilado nesse ponto, garantido
/// pela ordem topológica. `build` nunca re-resolve nem toca rede.
pub fn build(
    workspace: &Workspace,
    javac: &Path,
    cache_root: &Path,
) -> Result<Vec<ModuleBuildSummary>, BuildError> {
    let cache_store = CacheStore::new(cache_root);
    let dependency_classpath = classpath::locked_classpath(&workspace.lockfile, &cache_store)?;

    let order = order::module_order(&workspace.modules)?;

    let mut classes_dirs: HashMap<&str, PathBuf> = HashMap::with_capacity(workspace.modules.len());
    let mut summaries = Vec::with_capacity(workspace.modules.len());
    for index in order {
        let module = &workspace.modules[index];
        let module_sources = sources::collect_java_sources(&module.root.join("src/main/java"))?;
        let classes_dir = module.root.join("target/classes");

        let mut classpath = dependency_classpath.clone();
        for dependency_name in &module.workspace_dependencies {
            let dependency_classes_dir = classes_dirs
                .get(dependency_name.as_str())
                .expect("module_order guarantees every workspace dependency is built first");
            classpath.push(dependency_classes_dir.clone());
        }

        compile::compile(javac, &module_sources, &classpath, &classes_dir)?;
        let copied_resources =
            resources::copy_resources(&module.root.join("src/main/resources"), &classes_dir)?;

        classes_dirs.insert(module.name.as_str(), classes_dir.clone());
        summaries.push(ModuleBuildSummary {
            module: module.name.clone(),
            compiled_files: module_sources.len(),
            copied_resources,
            classes_dir,
        });
    }

    Ok(summaries)
}
