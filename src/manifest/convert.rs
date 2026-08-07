use super::dto::{DependencyValue, ProjectManifest};
use super::error::ManifestError;
use crate::domain::module::{BomReference, Dependency, Module, VersionReq};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn to_module(manifest: ProjectManifest, root: PathBuf) -> Result<Module, ManifestError> {
    Ok(Module {
        name: manifest.project.name,
        root,
        declared_dependencies: convert_dependencies(manifest.dependencies)?,
        boms: convert_boms(manifest.boms)?,
        exclusions: convert_exclusions(manifest.exclusions)?,
    })
}

/// `[dev-dependencies]` (seção 3) como um `Module` sintético separado —
/// nunca mesclado com o `Module` de produção, já que dev-deps só entram no
/// classpath de `jvmfast test` (seção 8.1), nunca em `build`/`run`.
/// Reaproveita os mesmos `[boms]`/`[exclusions]` do manifesto (são
/// configuração do projeto como um todo, não específicas de
/// `[dependencies]`). `None` quando `[dev-dependencies]` está vazio, para
/// `jvmfast test` pular resolução inteiramente — nunca um `Module` vazio
/// fabricado só para preencher o tipo.
pub fn to_dev_module(
    manifest: ProjectManifest,
    root: PathBuf,
) -> Result<Option<Module>, ManifestError> {
    if manifest.dev_dependencies.is_empty() {
        return Ok(None);
    }
    Ok(Some(Module {
        name: format!("{}-test", manifest.project.name),
        root,
        declared_dependencies: convert_dependencies(manifest.dev_dependencies)?,
        boms: convert_boms(manifest.boms)?,
        exclusions: convert_exclusions(manifest.exclusions)?,
    }))
}

fn convert_dependencies(
    dependencies: HashMap<String, DependencyValue>,
) -> Result<Vec<Dependency>, ManifestError> {
    let mut declared_dependencies = Vec::new();
    for (coordinate, value) in dependencies {
        validate_coordinate(&coordinate)?;
        let version_req = match value {
            DependencyValue::Explicit(v) => VersionReq::Explicit(v),
            DependencyValue::BomManaged => VersionReq::BomManaged,
        };
        declared_dependencies.push(Dependency {
            coordinate,
            version_req,
        });
    }
    Ok(declared_dependencies)
}

fn convert_boms(boms: HashMap<String, String>) -> Result<Vec<BomReference>, ManifestError> {
    let mut result = Vec::new();
    for (coordinate, version) in boms {
        validate_coordinate(&coordinate)?;
        result.push(BomReference {
            coordinate,
            version,
        });
    }
    Ok(result)
}

fn convert_exclusions(
    exclusions: HashMap<String, Vec<String>>,
) -> Result<HashMap<String, Vec<String>>, ManifestError> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (owner_coordinate, excluded) in exclusions {
        validate_coordinate(&owner_coordinate)?;
        for excl in &excluded {
            validate_coordinate(excl)?;
        }
        result.insert(owner_coordinate, excluded);
    }
    Ok(result)
}

/// Regra mínima que a arquitetura de fato especifica (seção 9.3): formato
/// `groupId:artifactId`. Não inventa regex de Maven além disso.
fn validate_coordinate(coordinate: &str) -> Result<(), ManifestError> {
    match coordinate.split_once(':') {
        Some((group, artifact)) if !group.is_empty() && !artifact.is_empty() => Ok(()),
        _ => Err(ManifestError::InvalidCoordinate(coordinate.to_string())),
    }
}
