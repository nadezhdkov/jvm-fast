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
/// aqui), e por fim invoca o Console Launcher (`console::run`). Itera
/// `workspace.modules`, mesma regra de compatibilidade com Fase 5 que
/// `build::build` já segue. `repo_base_url` (o `[repositories].default` do
/// projeto) resolve `[dev-dependencies]`; `console_base_url` (sempre
/// Maven Central — ver `console::ensure_console_jar`) baixa a ferramenta
/// interna, deliberadamente independente do repositório do projeto.
#[allow(clippy::too_many_arguments)]
pub async fn run_tests(
    workspace: &Workspace,
    dev_module: Option<&Module>,
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

    let mut classpath = dependency_classpath;
    classpath.extend(dev_classpath);
    classpath.push(console_jar.clone());

    let mut compiled_test_files = 0;
    let mut copied_test_resources = 0;
    let mut scan_dirs: Vec<PathBuf> = Vec::new();

    for module in &workspace.modules {
        let production_classes_dir = module.root.join("target/classes");
        let module_classpath: Vec<PathBuf> = classpath
            .iter()
            .cloned()
            .chain(std::iter::once(production_classes_dir.clone()))
            .collect();

        let test_sources = crate::build::collect_java_sources(&module.root.join("src/test/java"))?;
        let test_classes_dir = module.root.join("target/test-classes");
        crate::build::compile(javac, &test_sources, &module_classpath, &test_classes_dir)?;
        copied_test_resources += crate::build::copy_resources(
            &module.root.join("src/test/resources"),
            &test_classes_dir,
        )?;
        compiled_test_files += test_sources.len();

        classpath.push(production_classes_dir);
        classpath.push(test_classes_dir.clone());
        scan_dirs.push(test_classes_dir);
    }

    let exit_status = console::run(
        java,
        &console_jar,
        &classpath,
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
