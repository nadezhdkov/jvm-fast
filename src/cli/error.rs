use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Workspace(#[from] crate::workspace::WorkspaceLoadError),

    #[error(transparent)]
    Resolve(#[from] crate::resolve::ResolveError),

    #[error(transparent)]
    Download(#[from] crate::download::DownloadError),

    #[error(transparent)]
    Lockfile(#[from] crate::lockfile::LockfileError),

    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),

    #[error(transparent)]
    ManifestEdit(#[from] crate::cli::edit::ManifestEditError),

    #[error(transparent)]
    MavenLayout(#[from] crate::maven::MavenLayoutError),

    #[error(transparent)]
    Jdk(#[from] crate::jdk::JdkError),

    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    #[error(transparent)]
    Build(#[from] crate::build::BuildError),

    #[error(transparent)]
    Run(#[from] crate::run::RunError),

    #[error(transparent)]
    Testing(#[from] crate::testing::TestError),

    #[error(transparent)]
    Import(#[from] crate::import::ImportError),

    #[error(transparent)]
    GradleImport(#[from] crate::gradleimport::GradleImportError),

    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("`{0}` is not declared in [dependencies] — nothing to remove")]
    DependencyNotDeclared(String),

    #[error("`jvmfast add` without an explicit version (`{0}`) is not supported yet — repository metadata lookup for \"latest release\" (seção 9.3) isn't implemented; pass `coordinate@version`")]
    VersionOmittedNotSupported(String),

    #[error("`jvmfast add --dev` is not supported yet — editing [dev-dependencies] from the CLI isn't implemented (jvmfast test does resolve [dev-dependencies] declared directly in project.toml, see manifest::parse_dev_module)")]
    DevDependenciesNotSupported,

    #[error("`jvmfast update <coordinate>` (targeted update) is not supported yet — only a full re-resolution (`jvmfast update`) is")]
    TargetedUpdateNotSupported,

    #[error("coordinate `{0}` was not found in the resolved dependency graph")]
    CoordinateNotResolved(String),

    #[error("no module named `{0}` in this workspace")]
    ModuleNotFound(String),

    #[error("{0} of {1} artifact download(s) failed")]
    DownloadsFailed(usize, usize),

    #[error("Java {0} is not installed — run `jvmfast jdk install {0}` first")]
    JavaVersionNotInstalled(String),

    #[error("Java {0} is required by project.toml but is not installed — declined automatic install (pass `--yes` to install non-interactively)")]
    JdkInstallDeclined(String),

    #[error("no project.lock found — run `jvmfast install` before building/running/testing")]
    LockfileMissing,

    #[error(
        "project.lock is stale (project.toml changed since it was generated) — run `jvmfast install` or `jvmfast update` before building/running/testing"
    )]
    LockfileStale,

    #[error(
        "no [run].main-class configured in project.toml — `jvmfast run` needs one to know what to execute"
    )]
    MainClassNotConfigured,

    #[error("program exited with status {0}")]
    ProgramExited(i32),

    #[error(
        "`jvmfast test --fail-fast` is not supported yet — the JUnit Platform Console Launcher has no native stop-on-first-failure flag to map it to"
    )]
    FailFastNotSupported,

    #[error("tests failed (JUnit Platform Console Launcher exited with status {0})")]
    TestsFailed(i32),
}
