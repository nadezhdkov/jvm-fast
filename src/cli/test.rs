use crate::cli::context::{cache_root, jdks_root, resolve_base_url, MAVEN_CENTRAL};
use crate::cli::CliError;
use crate::download::DownloadClient;
use crate::jdk::find_installed;
use crate::lockfile::is_lockfile_valid;
use crate::manifest::parse_dev_module;
use crate::testing::{parse_filter, run_tests};
use crate::workspace::{current_manifest_hash, load_workspace};
use std::path::Path;

pub struct TestOptions {
    pub filter: Option<String>,
    pub fail_fast: bool,
    pub report_xml: bool,
}

/// `jvmfast test` (seção 8.1): mesmas checagens de `cli::build`/`cli::run`
/// (lock presente e válido, JDK do projeto instalada), depois compila
/// `src/main/java` (o código sob teste precisa de `target/classes`
/// atualizado — mesmo raciocínio de sempre recompilar de `cli::run`) antes
/// de delegar a `testing::run_tests` para compilar `src/test/java` e
/// invocar o JUnit Platform Console Standalone.
pub async fn run(root: &Path, options: TestOptions) -> Result<String, CliError> {
    if options.fail_fast {
        return Err(CliError::FailFastNotSupported);
    }

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
    let jdk_dir = jdks_root().join(installed);
    let javac = jdk_dir.join("bin/javac");
    let java = jdk_dir.join("bin/java");

    crate::build::build(&workspace, &javac, &cache_root())?;

    let dev_module = parse_dev_module(&root.join("project.toml"))?;
    let base_url = resolve_base_url(root)?;
    let download_client = DownloadClient::new(&workspace.config.network)?;
    let max_concurrent = workspace.config.network.concurrent_downloads.max(1) as usize;

    let filter = options.filter.as_deref().map(parse_filter);
    let reports_dir = options.report_xml.then(|| root.join("target/test-reports"));

    let summary = run_tests(
        &workspace,
        dev_module.as_ref(),
        &javac,
        &java,
        &cache_root(),
        &base_url,
        MAVEN_CENTRAL,
        &download_client,
        max_concurrent,
        filter.as_ref(),
        reports_dir.as_deref(),
    )
    .await?;

    if !summary.exit_status.success() {
        return Err(CliError::TestsFailed(
            summary.exit_status.code().unwrap_or(-1),
        ));
    }

    Ok(format!(
        "compiled {} test file(s), copied {} test resource(s) — all tests passed",
        summary.compiled_test_files, summary.copied_test_resources
    ))
}
