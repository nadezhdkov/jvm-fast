mod error;

pub use error::ConfigError;

use crate::domain::DefaultsConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// `~/.config/jvmfast/config.toml` (docs/architecture.md seção 3.5).
///
/// **Escopo desta passada**: só a leitura/escrita de `[defaults]` — o
/// consumidor real é `jvmfast jdk use`/`jdk list` (seção 7). `[network]`/
/// `[output]` (também documentados na seção 3.5) continuam não lidos por
/// ninguém; `workspace::load_workspace` ainda usa só
/// `WorkspaceConfig::default()` hardcoded — sobrepor esses defaults com o
/// arquivo global de verdade é um marco à parte, maior (a cadeia de
/// precedência completa da seção 3.5), não implementado aqui.
pub fn config_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/jvmfast/config.toml")
}

#[derive(Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    defaults: DefaultsConfig,
}

/// Lê `[defaults]` do config global — arquivo ausente é "nenhum override
/// ainda" (estado honesto, seção 3.5 não documenta um default hardcoded
/// para `java-version`/`repository`), não um erro.
pub fn load_defaults(path: &Path) -> Result<DefaultsConfig, ConfigError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DefaultsConfig::default()),
        Err(source) => {
            return Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            })
        }
    };

    let config: ConfigFile = toml::from_str(&contents).map_err(|source| ConfigError::Toml {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(config.defaults)
}

/// Grava `[defaults].java-version` (`jvmfast jdk use`, seção 7) editando o
/// arquivo no lugar via `toml_edit` — preserva `[network]`/`[output]` e
/// qualquer comentário, mesmo raciocínio de `cli::edit::add_dependency`
/// para `project.toml`. Cria o arquivo (e `~/.config/jvmfast/`, se
/// preciso) quando nenhum dos dois existe ainda.
pub fn write_default_java_version(path: &Path, version: &str) -> Result<(), ConfigError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            })
        }
    };

    let mut doc = contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|source| ConfigError::TomlEdit {
            path: path.to_path_buf(),
            source,
        })?;
    doc["defaults"]["java-version"] = toml_edit::value(version);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, doc.to_string()).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })
}
