mod console;
mod devdeps;
mod error;
mod filter;

pub use console::{CONSOLE_COORDINATE, CONSOLE_VERSION};
pub use error::TestError;
pub use filter::{glob_to_regex, parse_filter, TestFilter};

use crate::cache::CacheStore;
use crate::domain::{Module, Workspace};
use crate::download::DownloadClient;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::Arc;

#[derive(Debug)]
pub struct TestRunSummary {
    pub compiled_test_files: usize,
    pub copied_test_resources: usize,
    pub exit_status: ExitStatus,
}

/// `jvmfast test` (seção 8.1) ponta a ponta: resolve+baixa
/// `[dev-dependencies]` (`devdeps::resolve_dev_classpath`, `None` quando o
/// manifesto não declara nenhuma), garante o JUnit Platform Console
/// Standalone no cache (`console::ensure_console_jar` — "dependência
/// interna do jvm-fast", nunca em `project.toml`), compila `src/test/java`
/// contra `target/classes` (produção) + dependências de produção +
/// dev-deps + o próprio console jar, copia `src/test/resources` para
/// `target/test-classes` (nunca entra no classpath de `build`/`run`, só
/// aqui), e por fim invoca o Console Launcher (`console::run`).
///
/// `target_module` (seção 12, Fase 5: `jvmfast test --module <nome>`)
/// restringe compilação/execução a um único módulo — `None` preserva o
/// comportamento anterior (todos os módulos, `[dev-dependencies]` sempre
/// do módulo raiz — decisão de `cli::test`, não desta função). `Some(name)`
/// sem correspondência em `workspace.modules` é `TestError::UnknownModule`,
/// verificado antes de compilar qualquer coisa, não depois de um loop que
/// silenciosamente não processaria nenhum módulo.
///
/// Itera `workspace.modules` em ordem topológica (`build::module_order`,
/// mesma função que `build::build` já usa, seção 12 Fase 5) e, no
/// classpath de *compilação* de cada módulo, inclui só o `target/classes`
/// dele mesmo mais o de cada `Module.workspace_dependencies` declarado —
/// simetria explícita com `build::build`, não o acúmulo implícito "todo
/// módulo já processado antes" que esta função tinha antes (o que fazia um
/// módulo B enxergar as classes de A mesmo sem declarar dependência
/// nenhuma nele, só por A ter sido processado primeiro). O classpath de
/// *execução* passado ao Console Launcher continua incluindo produção +
/// teste de todos os módulos *processados nesta chamada* (todos, ou só o
/// selecionado via `target_module`) — o JUnit precisa resolver classes em
/// tempo de execução independente de qual módulo declarou o quê, então não
/// há por que restringir isso à mesma granularidade do classpath de
/// compilação. `repo_base_url` (o `[repositories].default` do projeto)
/// resolve `[dev-dependencies]`; `console_base_url` (sempre Maven Central —
/// ver `console::ensure_console_jar`) baixa a ferramenta interna,
/// deliberadamente independente do repositório do projeto.
#[allow(clippy::too_many_arguments)]
pub async fn run_tests(
    workspace: &Workspace,
    dev_module: Option<&Module>,
    target_module: Option<&str>,
    javac: &Path,
    java: &Path,
    cache_root: &Path,
    repo_base_url: &str,
    console_base_url: &str,
    download_client: &DownloadClient,
    max_concurrent: usize,
    filter: Option<&TestFilter>,
    reports_dir: Option<&Path>,
) -> Result<TestRunSummary, TestError> {
    if let Some(name) = target_module {
        if !workspace.modules.iter().any(|module| module.name == name) {
            return Err(TestError::UnknownModule(name.to_string()));
        }
    }

    let cache_store = Arc::new(CacheStore::new(cache_root));

    let dependency_classpath = crate::build::locked_classpath(&workspace.lockfile, &cache_store)?;
    let dev_classpath = match dev_module {
        Some(module) => {
            devdeps::resolve_dev_classpath(
                module,
                repo_base_url,
                Arc::clone(&cache_store),
                download_client,
                max_concurrent,
            )
            .await?
        }
        None => Vec::new(),
    };
    let console_jar =
        console::ensure_console_jar(download_client, &cache_store, console_base_url).await?;

    let mut base_classpath = dependency_classpath;
    base_classpath.extend(dev_classpath);
    base_classpath.push(console_jar.clone());

    let order = crate::build::module_order(&workspace.modules)?;
    let production_classes_dirs: HashMap<&str, PathBuf> = workspace
        .modules
        .iter()
        .map(|module| (module.name.as_str(), module.root.join("target/classes")))
        .collect();

    let mut compiled_test_files = 0;
    let mut copied_test_resources = 0;
    let mut scan_dirs: Vec<PathBuf> = Vec::new();
    // Classpath passado ao Console Launcher (execução, não compilação) —
    // sempre acumula produção + teste de todo módulo processado, já que a
    // JVM que executa os testes precisa resolver classes de qualquer
    // módulo do workspace independentemente de quem declarou dependência
    // de quem (ver doc comment acima).
    let mut run_classpath = base_classpath.clone();

    for index in order {
        let module = &workspace.modules[index];
        if let Some(name) = target_module {
            if module.name != name {
                continue;
            }
        }
        let production_classes_dir = production_classes_dirs[module.name.as_str()].clone();

        let mut module_classpath = base_classpath.clone();
        module_classpath.push(production_classes_dir.clone());
        for dependency_name in &module.workspace_dependencies {
            module_classpath.push(production_classes_dirs[dependency_name.as_str()].clone());
        }

        let test_sources = crate::build::collect_java_sources(&module.root.join("src/test/java"))?;
        let test_classes_dir = module.root.join("target/test-classes");
        crate::build::compile(javac, &test_sources, &module_classpath, &test_classes_dir)?;
        copied_test_resources += crate::build::copy_resources(
            &module.root.join("src/test/resources"),
            &test_classes_dir,
        )?;
        compiled_test_files += test_sources.len();

        run_classpath.push(production_classes_dir);
        run_classpath.push(test_classes_dir.clone());
        scan_dirs.push(test_classes_dir);
    }

    let exit_status = console::run(
        java,
        &console_jar,
        &run_classpath,
        &scan_dirs,
        filter,
        reports_dir,
    )?;

    Ok(TestRunSummary {
        compiled_test_files,
        copied_test_resources,
        exit_status,
    })
}
