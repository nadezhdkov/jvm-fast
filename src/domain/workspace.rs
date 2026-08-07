use super::lockfile::Lockfile;
use super::module::Module;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// O único ponto de entrada de resolução (docs/architecture.md seção 3.1,
/// 6.2). `Workspace::load` (`crate::workspace`) é o primeiro construtor
/// real, a partir do marco de lockfile.
#[derive(Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub modules: Vec<Module>,
    pub lockfile: Lockfile,
    pub config: WorkspaceConfig,
}

/// Espelha `~/.config/jvmfast/config.toml` (docs/architecture.md seção
/// 3.5). `Default` usa exatamente os valores default documentados na seção
/// 3.5 — não lê o arquivo real ainda (isso é um marco futuro, que passa a
/// sobrepor estes defaults, não a inventá-los).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceConfig {
    pub defaults: DefaultsConfig,
    pub network: NetworkConfig,
    pub output: OutputConfig,
}

/// Sem default documentado na seção 3.5 para `java-version`/`repository` —
/// são preferências do usuário, nunca inventadas por nós; `None` é a
/// ausência honesta de override, não um valor fabricado. Deriva
/// `Serialize`/`Deserialize` porque `config::load_defaults`/`jdk use`
/// (marco de `jdk use`) leem e escrevem exatamente esta seção de
/// `~/.config/jvmfast/config.toml` — o resto do arquivo (`[network]`,
/// `[output]`) ainda não é lido por ninguém.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(rename = "java-version", skip_serializing_if = "Option::is_none")]
    pub java_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkConfig {
    pub proxy: Option<String>,
    pub connect_timeout_secs: u32,
    pub max_retries: u32,
    pub concurrent_downloads: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            proxy: None,
            connect_timeout_secs: 10,
            max_retries: 3,
            // seção 3.5: "default = num_cpus" — usa a contagem real de
            // cores via std, sem depender de uma crate externa `num_cpus`.
            concurrent_downloads: std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(4),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConfig {
    pub color: ColorMode,
    pub progress_bar: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            color: ColorMode::Auto,
            progress_bar: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}
