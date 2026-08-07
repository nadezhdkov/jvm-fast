use super::error::BuildError;
use std::path::Path;

/// Copia `src_dir` (ex. `src/main/resources`) para dentro de `out_dir`
/// (`target/classes`), preservando estrutura relativa — seção 8:
/// `target/classes` é o resultado compilável e executável do módulo,
/// contendo tanto `.class` quanto recursos, sem etapa de merge separada.
/// `src_dir` ausente é zero recursos copiados, não erro (recursos são
/// opcionais).
pub fn copy_resources(src_dir: &Path, out_dir: &Path) -> Result<usize, BuildError> {
    let mut count = 0;
    if src_dir.is_dir() {
        copy_dir_recursive(src_dir, out_dir, &mut count)?;
    }
    Ok(count)
}

fn copy_dir_recursive(src: &Path, dst: &Path, count: &mut usize) -> Result<(), BuildError> {
    std::fs::create_dir_all(dst).map_err(|source| BuildError::Io {
        path: dst.to_path_buf(),
        source,
    })?;

    for entry in std::fs::read_dir(src).map_err(|source| BuildError::Io {
        path: src.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| BuildError::Io {
            path: src.to_path_buf(),
            source,
        })?;
        let from = entry.path();
        let to = dst.join(entry.file_name());

        if from.is_dir() {
            copy_dir_recursive(&from, &to, count)?;
        } else {
            std::fs::copy(&from, &to).map_err(|source| BuildError::Io {
                path: from.clone(),
                source,
            })?;
            *count += 1;
        }
    }
    Ok(())
}
