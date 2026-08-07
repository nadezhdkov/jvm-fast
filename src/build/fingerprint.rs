use super::error::BuildError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Nome do arquivo marcador dentro de `target/classes` que registra o
/// fingerprint do último build bem-sucedido do módulo — nunca escrito em
/// caso de falha de compilação (seção 12, Fase 5: "recompilar só módulos
/// afetados por uma mudança"). Prefixado com `.` para não colidir com
/// nenhum recurso real copiado de `src/main/resources` nem com nenhum
/// `.class` gerado.
pub const FINGERPRINT_FILE_NAME: &str = ".jvmfast-build-fingerprint";

/// Hash de conteúdo determinístico dos insumos de build de um módulo
/// (seção 12: "hash de conteúdo por módulo... aplicado a outputs de
/// compilação, não só a artefatos baixados") — cobre o conteúdo de cada
/// arquivo-fonte e recurso (não só timestamps, que mentem em checkouts
/// git/CI), o classpath completo (externo + dependências de workspace já
/// compiladas), o path do `javac` usado (uma troca de JDK via `jvmfast jdk
/// use` deve invalidar o cache), e o fingerprint de cada dependência de
/// workspace declarada — propagando invalidação transitiva sem precisar
/// re-hashear o conteúdo dessas dependências aqui (`build::build` já as
/// processa primeiro, em ordem topológica).
///
/// Toda lista é ordenada antes de entrar no hash — a ordem de iteração do
/// filesystem não é garantida, e o fingerprint precisa ser o mesmo para o
/// mesmo conjunto de arquivos independente de como o SO os listou.
pub fn compute_module_fingerprint(
    sources: &[PathBuf],
    resources_dir: &Path,
    classpath: &[PathBuf],
    javac: &Path,
    dependency_fingerprints: &[String],
) -> Result<String, BuildError> {
    let mut hasher = Sha256::new();

    let mut sorted_sources = sources.to_vec();
    sorted_sources.sort();
    for path in &sorted_sources {
        hash_file(&mut hasher, path)?;
    }

    let mut resource_files = collect_files(resources_dir)?;
    resource_files.sort();
    for path in &resource_files {
        hash_file(&mut hasher, path)?;
    }

    let mut classpath_entries: Vec<String> = classpath
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    classpath_entries.sort();
    for entry in &classpath_entries {
        hasher.update(entry.as_bytes());
        hasher.update(b"\0");
    }

    hasher.update(javac.to_string_lossy().as_bytes());

    let mut sorted_dependency_fingerprints = dependency_fingerprints.to_vec();
    sorted_dependency_fingerprints.sort();
    for fingerprint in &sorted_dependency_fingerprints {
        hasher.update(fingerprint.as_bytes());
    }

    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn hash_file(hasher: &mut Sha256, path: &Path) -> Result<(), BuildError> {
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    let contents = std::fs::read(path).map_err(|source| BuildError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    hasher.update(&contents);
    Ok(())
}

/// Lista recursiva de todo arquivo sob `dir` (não só `.java`, diferente de
/// `sources::collect_java_sources` — recursos podem ter qualquer
/// extensão). Diretório ausente é lista vazia, mesmo raciocínio de
/// `resources::copy_resources`.
fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
    let mut files = Vec::new();
    if dir.is_dir() {
        visit(dir, &mut files)?;
    }
    Ok(files)
}

fn visit(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), BuildError> {
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
            visit(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// Lê o fingerprint gravado do último build bem-sucedido, se houver —
/// `None` (nunca um erro) tanto para "nunca buildado" quanto para "arquivo
/// corrompido/ilegível", já que ambos os casos devem só forçar um rebuild,
/// nunca falhar `jvmfast build` por causa de um marcador de cache.
pub fn read_stored_fingerprint(classes_dir: &Path) -> Option<String> {
    std::fs::read_to_string(classes_dir.join(FINGERPRINT_FILE_NAME))
        .ok()
        .map(|contents| contents.trim().to_string())
}

/// Grava o fingerprint atômicamente (temp file → rename, seção 5.1) dentro
/// de `classes_dir` — só chamado depois de compilação e cópia de recursos
/// bem-sucedidas, nunca antes, para que uma falha no meio do caminho jamais
/// deixe um fingerprint "de sucesso" registrado para um build que não
/// terminou.
pub fn write_fingerprint(classes_dir: &Path, fingerprint: &str) -> Result<(), BuildError> {
    let final_path = classes_dir.join(FINGERPRINT_FILE_NAME);
    let temp_path = classes_dir.join(format!(
        "{FINGERPRINT_FILE_NAME}.part-{}",
        std::process::id()
    ));
    std::fs::write(&temp_path, fingerprint).map_err(|source| BuildError::Io {
        path: temp_path.clone(),
        source,
    })?;
    std::fs::rename(&temp_path, &final_path).map_err(|source| BuildError::Io {
        path: final_path,
        source,
    })?;
    Ok(())
}
