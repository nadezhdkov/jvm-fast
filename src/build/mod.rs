mod classpath;
mod compile;
mod error;
mod fingerprint;
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
    /// `true` quando o fingerprint dos insumos de build (fontes, recursos,
    /// classpath, `javac`, fingerprints das dependências de workspace)
    /// bate com o do último build bem-sucedido — nesse caso `compile`/
    /// `copy_resources` nunca rodam de novo, e `compiled_files`/
    /// `copied_resources` acima são `0` (nada foi de fato recompilado ou
    /// recopiado nesta chamada, não "o módulo não tem nada"). Seção 12,
    /// Fase 5: "recompilar só módulos afetados por uma mudança" — no nível
    /// de módulo inteiro, não arquivo-a-arquivo dentro de um módulo (isso
    /// continua sendo o gap já documentado desde a Fase 3).
    pub up_to_date: bool,
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
///
/// Build incremental por módulo (seção 12, Fase 5): antes de compilar,
/// calcula um fingerprint de conteúdo (`fingerprint::compute_module_fingerprint`)
/// dos insumos do módulo e compara contra o gravado no último build
/// bem-sucedido (`target/classes/.jvmfast-build-fingerprint`) — se
/// baterem, pula `compile`/`copy_resources` inteiramente
/// (`ModuleBuildSummary.up_to_date = true`). O fingerprint de cada
/// dependência de workspace entra no cálculo do fingerprint de quem
/// depende dela, propagando invalidação transitiva sem precisar re-hashear
/// o conteúdo da dependência de novo. Qualquer incerteza (fingerprint
/// ausente/ilegível, `target/classes` ausente) força rebuild — o cache
/// nunca é tratado como fonte de verdade (mesmo princípio de `src/cache`).
pub fn build(
    workspace: &Workspace,
    javac: &Path,
    cache_root: &Path,
) -> Result<Vec<ModuleBuildSummary>, BuildError> {
    let cache_store = CacheStore::new(cache_root);
    let dependency_classpath = classpath::locked_classpath(&workspace.lockfile, &cache_store)?;

    let order = order::module_order(&workspace.modules)?;

    let mut classes_dirs: HashMap<&str, PathBuf> = HashMap::with_capacity(workspace.modules.len());
    let mut fingerprints: HashMap<&str, String> = HashMap::with_capacity(workspace.modules.len());
    let mut summaries = Vec::with_capacity(workspace.modules.len());
    for index in order {
        let module = &workspace.modules[index];
        let module_sources = sources::collect_java_sources(&module.root.join("src/main/java"))?;
        let classes_dir = module.root.join("target/classes");
        let resources_dir = module.root.join("src/main/resources");

        let mut classpath = dependency_classpath.clone();
        for dependency_name in &module.workspace_dependencies {
            let dependency_classes_dir = classes_dirs
                .get(dependency_name.as_str())
                .expect("module_order guarantees every workspace dependency is built first");
            classpath.push(dependency_classes_dir.clone());
        }

        let dependency_fingerprints: Vec<String> = module
            .workspace_dependencies
            .iter()
            .map(|name| fingerprints[name.as_str()].clone())
            .collect();
        let module_fingerprint = fingerprint::compute_module_fingerprint(
            &module_sources,
            &resources_dir,
            &classpath,
            javac,
            &dependency_fingerprints,
        )?;

        let up_to_date = classes_dir.is_dir()
            && fingerprint::read_stored_fingerprint(&classes_dir).as_deref()
                == Some(module_fingerprint.as_str());

        let (compiled_files, copied_resources) = if up_to_date {
            (0, 0)
        } else {
            compile::compile(javac, &module_sources, &classpath, &classes_dir)?;
            let copied_resources = resources::copy_resources(&resources_dir, &classes_dir)?;
            fingerprint::write_fingerprint(&classes_dir, &module_fingerprint)?;
            (module_sources.len(), copied_resources)
        };

        classes_dirs.insert(module.name.as_str(), classes_dir.clone());
        fingerprints.insert(module.name.as_str(), module_fingerprint);
        summaries.push(ModuleBuildSummary {
            module: module.name.clone(),
            compiled_files,
            copied_resources,
            classes_dir,
            up_to_date,
        });
    }

    Ok(summaries)
}
