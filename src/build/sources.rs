use super::error::BuildError;
use std::path::{Path, PathBuf};

/// Coleta todos os `.java` sob `dir` (recursivo) — seção 8: layout padrão
/// `src/main/java`/`src/test/java`, sem exigir declaração individual de
/// arquivo no manifesto. Diretório ausente é lista vazia (módulo sem
/// código-fonte ainda é válido), não erro.
pub fn collect_java_sources(dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
    let mut sources = Vec::new();
    if dir.is_dir() {
        visit(dir, &mut sources)?;
        sources.sort();
    }
    Ok(sources)
}

fn visit(dir: &Path, sources: &mut Vec<PathBuf>) -> Result<(), BuildError> {
    for entry in std::fs::read_dir(dir).map_err(|source| BuildError::Io {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| BuildError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            visit(&path, sources)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("java") {
            sources.push(path);
        }
    }
    Ok(())
}
