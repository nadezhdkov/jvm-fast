use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestEditError {
    #[error("could not read manifest at {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid TOML in manifest at {path}: {source}")]
    Parse {
        path: std::path::PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },
}

/// Adiciona (ou substitui) uma entrada em `[dependencies]` (`jvmfast add`,
/// seção 9.3) editando `project.toml` no lugar via `toml_edit`, em vez de
/// desserializar para `ProjectManifest` e reserializar do zero — preserva
/// comentários e formatação do resto do arquivo, o mesmo motivo que faz o
/// próprio `cargo add` usar essa biblioteca.
pub fn add_dependency(
    manifest_path: &Path,
    coordinate: &str,
    version: &str,
) -> Result<(), ManifestEditError> {
    let mut doc = read_document(manifest_path)?;
    doc["dependencies"][coordinate] = toml_edit::value(version);
    write_document(manifest_path, &doc)
}

/// Remove uma entrada de `[dependencies]` (`jvmfast remove`). Retorna
/// `false` sem tocar o arquivo se a coordenada não estava declarada —
/// quem chama decide se isso é erro (CLI) ou não-operação silenciosa.
pub fn remove_dependency(
    manifest_path: &Path,
    coordinate: &str,
) -> Result<bool, ManifestEditError> {
    let mut doc = read_document(manifest_path)?;
    let removed = doc
        .get_mut("dependencies")
        .and_then(|deps| deps.as_table_like_mut())
        .is_some_and(|table| table.remove(coordinate).is_some());

    if removed {
        write_document(manifest_path, &doc)?;
    }
    Ok(removed)
}

fn read_document(path: &Path) -> Result<toml_edit::DocumentMut, ManifestEditError> {
    let contents = std::fs::read_to_string(path).map_err(|source| ManifestEditError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|source| ManifestEditError::Parse {
            path: path.to_path_buf(),
            source,
        })
}

fn write_document(path: &Path, doc: &toml_edit::DocumentMut) -> Result<(), ManifestEditError> {
    std::fs::write(path, doc.to_string()).map_err(|source| ManifestEditError::Io {
        path: path.to_path_buf(),
        source,
    })
}
