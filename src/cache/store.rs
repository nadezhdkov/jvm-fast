use super::error::CacheError;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// SHA-256 hexadecimal (minúsculo, sem prefixo) de `contents` — a mesma
/// forma que os artefatos publicados por repositórios Maven expõem em
/// arquivos `.sha256`.
pub fn hash_bytes(contents: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Armazenamento de artefatos content-addressable (seção 5) — o path de um
/// artefato é derivado do seu SHA-256, nunca do nome, para que múltiplos
/// projetos compartilhem o mesmo JAR fisicamente uma única vez no disco.
pub struct CacheStore {
    root: PathBuf,
}

impl CacheStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Sharding em 2 níveis (seção 5): `artifacts/sha256/a1/b2/a1b2c3.../<filename>`
    /// evita diretórios com dezenas de milhares de entradas.
    pub fn artifact_path(&self, sha256: &str, filename: &str) -> PathBuf {
        self.root
            .join("artifacts")
            .join("sha256")
            .join(&sha256[0..2])
            .join(&sha256[2..4])
            .join(sha256)
            .join(filename)
    }

    pub fn is_cached(&self, sha256: &str, filename: &str) -> bool {
        self.artifact_path(sha256, filename).is_file()
    }

    /// Escreve um artefato de forma atômica (seção 5.1): arquivo temporário
    /// no diretório final → verifica checksum → rename atômico. Um arquivo
    /// temporário incompleto nunca é uma entrada válida do cache — só existe
    /// entrada válida após o rename, indivisível no filesystem. Se o path
    /// final já existir, o conteúdo é considerado idêntico (garantido pelo
    /// hash fazer parte do próprio path) e a escrita é pulada — dois
    /// processos baixando o mesmo artefato ao mesmo tempo nunca se corrompem.
    pub fn write_artifact(
        &self,
        contents: &[u8],
        expected_sha256: &str,
        filename: &str,
    ) -> Result<PathBuf, CacheError> {
        let actual_sha256 = hash_bytes(contents);
        if actual_sha256 != expected_sha256 {
            return Err(CacheError::ChecksumMismatch {
                filename: filename.to_string(),
                expected: expected_sha256.to_string(),
                actual: actual_sha256,
            });
        }

        let final_path = self.artifact_path(expected_sha256, filename);
        if final_path.is_file() {
            return Ok(final_path);
        }

        let final_dir = final_path
            .parent()
            .expect("artifact_path always nests under a shard directory");
        std::fs::create_dir_all(final_dir).map_err(|source| CacheError::Io {
            path: final_dir.to_path_buf(),
            source,
        })?;

        let temp_path = final_dir.join(format!("{filename}.part-{}", std::process::id()));
        std::fs::write(&temp_path, contents).map_err(|source| CacheError::Io {
            path: temp_path.clone(),
            source,
        })?;
        std::fs::rename(&temp_path, &final_path).map_err(|source| CacheError::Io {
            path: final_path.clone(),
            source,
        })?;

        Ok(final_path)
    }
}
