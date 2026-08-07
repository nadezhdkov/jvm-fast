mod error;
mod generate;
mod range;

pub use error::ImportError;
pub use generate::{render_manifest, ImportedVersion};
pub use range::{is_maven_range, translate_maven_range, RangeTranslation};

use crate::pom::{parse_pom_xml, ManagedDependencyEntry, PomDependency};
use std::collections::HashMap;
use std::path::Path;

const JAVA_VERSION_PROPERTIES: &[&str] = &[
    "maven.compiler.release",
    "maven.compiler.target",
    "maven.compiler.source",
    "java.version",
];

/// Notas de elementos do POM sem equivalente direto em `project.toml`
/// (plugins, profiles, propriedades não resolvidas, ranges sem tradução
/// simples...) — docs/architecture.md seção 10: "reportando quais
/// elementos não têm equivalente e precisam de atenção manual". Nunca
/// falha o import inteiro; cada nota corresponde a algo que foi *omitido*
/// do manifesto gerado, não corrompido nele.
pub struct ImportReport {
    pub notes: Vec<String>,
}

/// Lê `pom_path` e escreve um `project.toml` novo em `manifest_path`
/// (docs/architecture.md seção 10, `jvmfast import-pom`). Nunca sobrescreve
/// um manifesto existente (`ImportError::ManifestAlreadyExists`) e nunca
/// escreve em `pom_path` — os dois arquivos podem coexistir durante a
/// transição, exatamente como a seção 10 documenta.
pub fn import_pom(pom_path: &Path, manifest_path: &Path) -> Result<ImportReport, ImportError> {
    if manifest_path.exists() {
        return Err(ImportError::ManifestAlreadyExists(
            manifest_path.to_path_buf(),
        ));
    }

    let xml = std::fs::read_to_string(pom_path).map_err(|source| ImportError::Io {
        path: pom_path.to_path_buf(),
        source,
    })?;
    let pom = parse_pom_xml(&xml)?;

    if pom.project_artifact_id.is_empty() {
        return Err(ImportError::MissingArtifactId);
    }
    if pom.project_version.is_empty() {
        return Err(ImportError::MissingVersion);
    }

    let mut notes = Vec::new();
    let java_version = resolve_java_version(&pom.properties, &mut notes);
    let source_encoding = pom.properties.get("project.build.sourceEncoding").cloned();
    let has_bom_import = pom.managed_dependencies.iter().any(|e| e.is_bom_import);

    let (dependencies, dev_dependencies, exclusions) = convert_dependencies(
        &pom.dependencies,
        &pom.properties,
        has_bom_import,
        &mut notes,
    );
    let boms = convert_boms(&pom.managed_dependencies, &pom.properties, &mut notes);
    let repositories = convert_repositories(&pom.repositories, &mut notes);

    if pom.has_profiles {
        notes.push(
            "pom.xml declares <profiles> — profiles have no jvm-fast equivalent and were not imported"
                .to_string(),
        );
    }
    if pom.has_plugins {
        notes.push(
            "pom.xml declares <build><plugins> — plugins have no jvm-fast equivalent and were not imported"
                .to_string(),
        );
    }

    let manifest = generate::render_manifest(
        &pom.project_artifact_id,
        &pom.project_version,
        &java_version,
        source_encoding.as_deref(),
        &dependencies,
        &dev_dependencies,
        &boms,
        &exclusions,
        &repositories,
    );

    std::fs::write(manifest_path, manifest).map_err(|source| ImportError::Io {
        path: manifest_path.to_path_buf(),
        source,
    })?;

    Ok(ImportReport { notes })
}

#[allow(clippy::type_complexity)]
fn convert_dependencies(
    deps: &[PomDependency],
    properties: &HashMap<String, String>,
    has_bom_import: bool,
    notes: &mut Vec<String>,
) -> (
    Vec<(String, ImportedVersion)>,
    Vec<(String, ImportedVersion)>,
    Vec<(String, Vec<String>)>,
) {
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    let mut exclusions = Vec::new();

    for dep in deps {
        if matches!(dep.scope.as_str(), "provided" | "system") {
            notes.push(format!(
                "dependency `{}`: scope `{}` has no jvm-fast equivalent — skipped, add manually if needed",
                dep.coordinate, dep.scope
            ));
            continue;
        }

        let Some(version) = resolve_dependency_version(dep, properties, has_bom_import, notes)
        else {
            continue;
        };

        if !dep.exclusions.is_empty() {
            exclusions.push((dep.coordinate.clone(), dep.exclusions.clone()));
        }

        if dep.scope == "test" {
            dev_dependencies.push((dep.coordinate.clone(), version));
        } else {
            dependencies.push((dep.coordinate.clone(), version));
        }
    }

    (dependencies, dev_dependencies, exclusions)
}

fn resolve_dependency_version(
    dep: &PomDependency,
    properties: &HashMap<String, String>,
    has_bom_import: bool,
    notes: &mut Vec<String>,
) -> Option<ImportedVersion> {
    if dep.version.is_empty() {
        if has_bom_import {
            return Some(ImportedVersion::BomManaged);
        }
        notes.push(format!(
            "dependency `{}`: no <version> and no imported BOM found in <dependencyManagement> — skipped, add a version manually",
            dep.coordinate
        ));
        return None;
    }

    let Some(interpolated) = interpolate(&dep.version, properties) else {
        notes.push(format!(
            "dependency `{}`: version property in `{}` could not be resolved — skipped, add a version manually",
            dep.coordinate, dep.version
        ));
        return None;
    };

    if is_maven_range(&interpolated) {
        match translate_maven_range(&interpolated) {
            RangeTranslation::Direct(version) => Some(ImportedVersion::Explicit(version)),
            RangeTranslation::Unresolved => {
                notes.push(format!(
                    "dependency `{}`: version range `{}` has no direct jvm-fast equivalent — skipped, pin a version manually (seção 10)",
                    dep.coordinate, interpolated
                ));
                None
            }
        }
    } else {
        Some(ImportedVersion::Explicit(interpolated))
    }
}

fn convert_boms(
    managed: &[ManagedDependencyEntry],
    properties: &HashMap<String, String>,
    notes: &mut Vec<String>,
) -> Vec<(String, String)> {
    managed
        .iter()
        .filter(|entry| entry.is_bom_import)
        .filter_map(|entry| {
            if entry.version.is_empty() {
                notes.push(format!(
                    "BOM `{}`: no <version> declared — skipped, add manually",
                    entry.coordinate
                ));
                return None;
            }
            match interpolate(&entry.version, properties) {
                Some(version) => Some((entry.coordinate.clone(), version)),
                None => {
                    notes.push(format!(
                        "BOM `{}`: version property in `{}` could not be resolved — skipped, add manually",
                        entry.coordinate, entry.version
                    ));
                    None
                }
            }
        })
        .collect()
}

fn convert_repositories(
    repositories: &[(String, String)],
    notes: &mut Vec<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (index, (id, url)) in repositories.iter().enumerate() {
        if index == 0 {
            out.push(("default".to_string(), url.clone()));
        } else {
            let key = if id.is_empty() {
                format!("repo-{index}")
            } else {
                id.clone()
            };
            out.push((key, url.clone()));
        }
    }
    if repositories.len() > 1 {
        notes.push(format!(
            "{} additional repositor{} imported into [repositories] beyond `default` — jvm-fast only resolves against `default` today, see CLAUDE.md",
            repositories.len() - 1,
            if repositories.len() - 1 == 1 { "y" } else { "ies" }
        ));
    }
    out
}

fn resolve_java_version(properties: &HashMap<String, String>, notes: &mut Vec<String>) -> String {
    for key in JAVA_VERSION_PROPERTIES {
        if let Some(value) = properties.get(*key) {
            if !value.is_empty() {
                return value.clone();
            }
        }
    }
    notes.push(
        "no maven.compiler.release/target/source or java.version property found in pom.xml — defaulted java-version to \"lts\", adjust if needed"
            .to_string(),
    );
    "lts".to_string()
}

/// Substitui `${chave}` por `properties[chave]`, uma ou mais vezes na
/// mesma string (ex. `${a}-${b}`). `None` se qualquer chave referenciada
/// não estiver em `properties` — herança de POM pai não é seguida (mesma
/// lacuna de `crate::pom::xml`), então uma propriedade herdada do pai
/// aparece aqui como "não resolvida", não como erro fatal do import
/// inteiro.
fn interpolate(raw: &str, properties: &HashMap<String, String>) -> Option<String> {
    let mut result = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_marker = &rest[start + 2..];
        let end = after_marker.find('}')?;
        let key = &after_marker[..end];
        let value = properties.get(key)?;
        result.push_str(value);
        rest = &after_marker[end + 1..];
    }
    result.push_str(rest);
    Some(result)
}
