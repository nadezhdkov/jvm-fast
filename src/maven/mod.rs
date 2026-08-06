mod error;

pub use error::MavenLayoutError;

/// Path relativo (sem barra inicial) de um artefato no layout padrão de
/// repositório Maven: `groupId` com pontos virados em barras, seguido de
/// `artifactId/version/artifactId-version.<extension>`. Compartilhado entre
/// `pom::HttpPomProvider` (fetch de `.pom`) e `download` (fetch de `.jar` e
/// do sidecar `.sha256`) para as duas superfícies HTTP nunca divergirem em
/// como calculam a mesma coisa.
pub fn artifact_path(
    coordinate: &str,
    version: &str,
    extension: &str,
) -> Result<String, MavenLayoutError> {
    let (group_id, artifact_id) = coordinate
        .split_once(':')
        .filter(|(g, a)| !g.is_empty() && !a.is_empty())
        .ok_or_else(|| MavenLayoutError::InvalidCoordinate(coordinate.to_string()))?;

    Ok(format!(
        "{}/{artifact_id}/{version}/{artifact_id}-{version}.{extension}",
        group_id.replace('.', "/"),
    ))
}

pub fn artifact_url(
    base_url: &str,
    coordinate: &str,
    version: &str,
    extension: &str,
) -> Result<String, MavenLayoutError> {
    let path = artifact_path(coordinate, version, extension)?;
    Ok(format!("{}/{path}", base_url.trim_end_matches('/')))
}

pub fn artifact_filename(
    coordinate: &str,
    version: &str,
    extension: &str,
) -> Result<String, MavenLayoutError> {
    let (_, artifact_id) = coordinate
        .split_once(':')
        .filter(|(g, a)| !g.is_empty() && !a.is_empty())
        .ok_or_else(|| MavenLayoutError::InvalidCoordinate(coordinate.to_string()))?;
    Ok(format!("{artifact_id}-{version}.{extension}"))
}
