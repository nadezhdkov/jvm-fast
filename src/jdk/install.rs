use super::adoptium::JdkRelease;
use super::error::JdkError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Diretório de uma JDK instalada (seção 5: `~/.cache/jvmfast/jdks/<versão>-tem/`).
pub fn jdk_install_dir(jdks_root: &Path, release: &JdkRelease) -> PathBuf {
    jdks_root.join(format!("{}-tem", release.version))
}

pub fn is_installed(jdks_root: &Path, release: &JdkRelease) -> bool {
    jdk_install_dir(jdks_root, release).is_dir()
}

/// Baixa, verifica o checksum e extrai o `.tar.gz` da JDK (seção 7).
/// Idempotente — se o diretório final já existe, não baixa de novo.
///
/// Extração é atômica: descompacta num diretório temporário dentro de
/// `jdks_root` e só faz `rename` para o path final depois de terminar —
/// mesma disciplina de `cache::CacheStore::write_artifact` (seção 5.1),
/// adaptada de um arquivo único para uma árvore de diretórios.
pub async fn install(
    client: &reqwest::Client,
    jdks_root: &Path,
    release: &JdkRelease,
) -> Result<PathBuf, JdkError> {
    let final_dir = jdk_install_dir(jdks_root, release);
    if is_installed(jdks_root, release) {
        return Ok(final_dir);
    }

    let response = client
        .get(&release.download_url)
        .send()
        .await
        .map_err(|source| JdkError::Request {
            url: release.download_url.clone(),
            source,
        })?;
    if !response.status().is_success() {
        return Err(JdkError::Status {
            url: release.download_url.clone(),
            status: response.status().as_u16(),
        });
    }
    let bytes = response.bytes().await.map_err(|source| JdkError::Request {
        url: release.download_url.clone(),
        source,
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual != release.checksum {
        return Err(JdkError::ChecksumMismatch {
            filename: release.filename.clone(),
            expected: release.checksum.clone(),
            actual,
        });
    }

    std::fs::create_dir_all(jdks_root).map_err(|source| JdkError::Io {
        path: jdks_root.to_path_buf(),
        source,
    })?;
    let temp_dir = jdks_root.join(format!(
        ".{}-tem.part-{}",
        release.version,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).map_err(|source| JdkError::Io {
        path: temp_dir.clone(),
        source,
    })?;

    extract_tar_gz(&bytes, &temp_dir)?;
    let inner = single_child_dir(&temp_dir)?;
    std::fs::rename(&inner, &final_dir).map_err(|source| JdkError::Io {
        path: final_dir.clone(),
        source,
    })?;
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(final_dir)
}

fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<(), JdkError> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).map_err(JdkError::Extract)
}

/// Distribuições Temurin extraem para um único diretório-raiz dentro do tar
/// (ex. `jdk-21.0.2+13/`) — subimos um nível para que o diretório final já
/// seja diretamente utilizável como `JAVA_HOME`.
fn single_child_dir(dir: &Path) -> Result<PathBuf, JdkError> {
    std::fs::read_dir(dir)
        .map_err(|source| JdkError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| {
            JdkError::Extract(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "archive did not contain a root directory",
            ))
        })
}
