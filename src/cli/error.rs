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

    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("`{0}` is not declared in [dependencies] — nothing to remove")]
    DependencyNotDeclared(String),

    #[error("`jvmfast add` without an explicit version (`{0}`) is not supported yet — repository metadata lookup for \"latest release\" (seção 9.3) isn't implemented; pass `coordinate@version`")]
    VersionOmittedNotSupported(String),

    #[error("`jvmfast add --dev` is not supported yet — dev-dependencies aren't threaded from the manifest into Module yet (see src/manifest/dto.rs)")]
    DevDependenciesNotSupported,

    #[error("`jvmfast update <coordinate>` (targeted update) is not supported yet — only a full re-resolution (`jvmfast update`) is")]
    TargetedUpdateNotSupported,

    #[error("coordinate `{0}` was not found in the resolved dependency graph")]
    CoordinateNotResolved(String),

    #[error("{0} of {1} artifact download(s) failed")]
    DownloadsFailed(usize, usize),

    #[error("Java {0} is not installed — run `jvmfast jdk install {0}` first")]
    JavaVersionNotInstalled(String),

    #[error("Java {0} is required by project.toml but is not installed — declined automatic install (pass `--yes` to install non-interactively)")]
    JdkInstallDeclined(String),

    #[error("no project.lock found — run `jvmfast install` before building/running")]
    LockfileMissing,

    #[error(
        "project.lock is stale (project.toml changed since it was generated) — run `jvmfast install` or `jvmfast update` before building/running"
    )]
    LockfileStale,

    #[error(
        "no [run].main-class configured in project.toml — `jvmfast run` needs one to know what to execute"
    )]
    MainClassNotConfigured,

    #[error("program exited with status {0}")]
    ProgramExited(i32),
}
