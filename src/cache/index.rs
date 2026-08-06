use super::error::CacheError;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// Índice SQLite de metadados de artefatos em cache (seção 5) — permite
/// responder "quais versões de X estão em cache" sem varrer o filesystem.
/// O índice nunca é fonte de verdade (seção 5.1): se ficar inconsistente com
/// o que existe fisicamente em `artifacts/`, a resposta é sempre remover e
/// reconstruir, nunca reparo em memória.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedArtifact {
    pub coordinate: String,
    pub version: String,
    pub sha256: String,
    pub filename: String,
}

/// Abre (criando se necessário) o `index.db` do cache, garantindo que o
/// schema exista. Leituras concorrentes são permitidas livremente; escritas
/// contam com as garantias transacionais do próprio SQLite (seção 5.1) — sem
/// locking manual adicional.
pub fn open_index(path: &Path) -> Result<Connection, CacheError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| CacheError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let conn = Connection::open(path).map_err(|source| CacheError::Index {
        context: format!("open index at {}", path.display()),
        source,
    })?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS artifacts (
            coordinate TEXT NOT NULL,
            version    TEXT NOT NULL,
            sha256     TEXT NOT NULL,
            filename   TEXT NOT NULL,
            PRIMARY KEY (coordinate, version)
        )",
        [],
    )
    .map_err(|source| CacheError::Index {
        context: "create artifacts table".to_string(),
        source,
    })?;

    Ok(conn)
}

/// Registra (ou atualiza) a entrada de um artefato em cache. `INSERT ...
/// ON CONFLICT DO UPDATE` em vez de checar existência antes — a própria
/// transação do SQLite resolve a corrida entre processos concorrentes.
pub fn record_artifact(conn: &Connection, artifact: &CachedArtifact) -> Result<(), CacheError> {
    conn.execute(
        "INSERT INTO artifacts (coordinate, version, sha256, filename)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(coordinate, version) DO UPDATE SET
             sha256 = excluded.sha256,
             filename = excluded.filename",
        params![
            artifact.coordinate,
            artifact.version,
            artifact.sha256,
            artifact.filename
        ],
    )
    .map(|_| ())
    .map_err(|source| CacheError::Index {
        context: format!(
            "record artifact {}@{}",
            artifact.coordinate, artifact.version
        ),
        source,
    })
}

pub fn find_artifact(
    conn: &Connection,
    coordinate: &str,
    version: &str,
) -> Result<Option<CachedArtifact>, CacheError> {
    conn.query_row(
        "SELECT coordinate, version, sha256, filename FROM artifacts
         WHERE coordinate = ?1 AND version = ?2",
        params![coordinate, version],
        |row| {
            Ok(CachedArtifact {
                coordinate: row.get(0)?,
                version: row.get(1)?,
                sha256: row.get(2)?,
                filename: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|source| CacheError::Index {
        context: format!("find artifact {coordinate}@{version}"),
        source,
    })
}

pub fn list_cached_versions(
    conn: &Connection,
    coordinate: &str,
) -> Result<Vec<String>, CacheError> {
    let mut statement = conn
        .prepare("SELECT version FROM artifacts WHERE coordinate = ?1 ORDER BY version")
        .map_err(|source| CacheError::Index {
            context: format!("list cached versions for {coordinate}"),
            source,
        })?;

    let rows = statement
        .query_map(params![coordinate], |row| row.get::<_, String>(0))
        .map_err(|source| CacheError::Index {
            context: format!("list cached versions for {coordinate}"),
            source,
        })?;

    let mut versions = Vec::new();
    for row in rows {
        versions.push(row.map_err(|source| CacheError::Index {
            context: format!("list cached versions for {coordinate}"),
            source,
        })?);
    }
    Ok(versions)
}
