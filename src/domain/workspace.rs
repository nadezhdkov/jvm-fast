use super::lockfile::Lockfile;
use super::module::Module;
use std::path::PathBuf;

/// O único ponto de entrada de resolução (docs/architecture.md seção 3.1,
/// 6.2). Declarado desde já para fixar o vocabulário de tipos, mas nenhum
/// construtor existe ainda nesta passada — `Lockfile` não tem fonte de
/// dados real até o marco de lockfile I/O.
pub struct Workspace {
    pub root: PathBuf,
    pub modules: Vec<Module>,
    pub lockfile: Lockfile,
    pub config: WorkspaceConfig,
}

/// Espelha `~/.config/jvmfast/config.toml` (docs/architecture.md seção
/// 3.5). Nada parseia esse arquivo ainda nesta passada.
pub struct WorkspaceConfig {
    pub defaults: DefaultsConfig,
    pub network: NetworkConfig,
    pub output: OutputConfig,
}

pub struct DefaultsConfig {
    pub java_version: Option<String>,
    pub repository: Option<String>,
}

pub struct NetworkConfig {
    pub proxy: Option<String>,
    pub connect_timeout_secs: u32,
    pub max_retries: u32,
    pub concurrent_downloads: u32,
}

pub struct OutputConfig {
    pub color: ColorMode,
    pub progress_bar: bool,
}

pub enum ColorMode {
    Auto,
    Always,
    Never,
}
