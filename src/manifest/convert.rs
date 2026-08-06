use super::dto::{DependencyValue, ProjectManifest};
use super::error::ManifestError;
use crate::domain::module::{BomReference, Dependency, Module, VersionReq};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn to_module(manifest: ProjectManifest, root: PathBuf) -> Result<Module, ManifestError> {
    let mut declared_dependencies = Vec::new();
    for (coordinate, value) in manifest.dependencies {
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

    let mut boms = Vec::new();
    for (coordinate, version) in manifest.boms {
        validate_coordinate(&coordinate)?;
        boms.push(BomReference {
            coordinate,
            version,
        });
    }

    let mut exclusions: HashMap<String, Vec<String>> = HashMap::new();
    for (owner_coordinate, excluded) in manifest.exclusions {
        validate_coordinate(&owner_coordinate)?;
        for excl in &excluded {
            validate_coordinate(excl)?;
        }
        exclusions.insert(owner_coordinate, excluded);
    }

    Ok(Module {
        name: manifest.project.name,
        root,
        declared_dependencies,
        boms,
        exclusions,
    })
}

/// Regra mínima que a arquitetura de fato especifica (seção 9.3): formato
/// `groupId:artifactId`. Não inventa regex de Maven além disso.
fn validate_coordinate(coordinate: &str) -> Result<(), ManifestError> {
    match coordinate.split_once(':') {
        Some((group, artifact)) if !group.is_empty() && !artifact.is_empty() => Ok(()),
        _ => Err(ManifestError::InvalidCoordinate(coordinate.to_string())),
    }
}
