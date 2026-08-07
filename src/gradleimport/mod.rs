mod error;
mod initscript;
mod model;

pub use error::GradleImportError;

use crate::import::{render_manifest, ImportReport, ImportedVersion};
use model::BridgeModel;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Reads a Gradle project at `project_dir` through the Tooling API bridge
/// (docs/architecture.md seção 10: `jvmfast-gradle-bridge.jar`) and writes
/// an equivalent `project.toml` at `manifest_path` — the `import-gradle`
/// counterpart to `crate::import::import_pom`. Never touches the source
/// Gradle build files, never overwrites an existing manifest
/// (`GradleImportError::ManifestAlreadyExists`).
///
/// Flow (seção 10, steps 1-4): generates a temporary init-script that
/// applies `JvmfastModelBuilderPlugin` (classpath = the embedded bridge jar
/// itself, extracted via `crate::gradlebridge`), then invokes that same jar
/// as `java -jar ... <project_dir> <init_script>` — its `Main` class opens
/// a Tooling API connection to `project_dir`'s own `gradlew`, requests the
/// typed `JvmfastDependencyModel`, and prints *only* the resulting JSON to
/// its stdout. This function parses that JSON and maps it onto
/// `project.toml`'s shape.
pub fn import_gradle(
    project_dir: &Path,
    manifest_path: &Path,
    cache_root: &Path,
) -> Result<ImportReport, GradleImportError> {
    if manifest_path.exists() {
        return Err(GradleImportError::ManifestAlreadyExists(
            manifest_path.to_path_buf(),
        ));
    }

    let gradlew_name = if cfg!(windows) {
        "gradlew.bat"
    } else {
        "gradlew"
    };
    if !project_dir.join(gradlew_name).is_file() {
        return Err(GradleImportError::GradlewNotFound(
            project_dir.to_path_buf(),
        ));
    }

    let bridge_jar = crate::gradlebridge::extract_bridge_jar(cache_root)?;
    let init_script =
        initscript::write_init_script(&bridge_jar).map_err(|source| GradleImportError::Io {
            path: std::env::temp_dir(),
            source,
        })?;

    let invocation = Command::new("java")
        .arg("-jar")
        .arg(&bridge_jar)
        .arg(project_dir)
        .arg(&init_script)
        .output();
    let _ = std::fs::remove_file(&init_script);
    let output = invocation.map_err(GradleImportError::JavaNotFound)?;

    if !output.status.success() {
        return Err(GradleImportError::BridgeFailed {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let model: BridgeModel = serde_json::from_slice(&output.stdout)?;
    let module = model
        .modules
        .first()
        .ok_or(GradleImportError::NoModulesInBridgeOutput)?;

    let mut notes = Vec::new();
    if model.modules.len() > 1 {
        notes.push(format!(
            "gradle build has {} modules — jvmfast import-gradle only imports the first one \
             today (multi-project import is Fase 5 scope)",
            model.modules.len()
        ));
    }

    let version = if module.version == "unspecified" {
        notes.push(
            "gradle project has no version set — defaulted to \"0.1.0\", adjust manually if needed"
                .to_string(),
        );
        "0.1.0".to_string()
    } else {
        module.version.clone()
    };

    notes.push(
        "java-version could not be read from the Gradle model (not exposed by \
         JvmfastDependencyModel yet) — defaulted to \"lts\", adjust manually if needed"
            .to_string(),
    );
    notes.push(
        "dependency versions were imported exactly as Gradle resolved them; a subsequent \
         `jvmfast update` may select different versions, since jvm-fast mediates conflicts by \
         nearest-depth-wins while Gradle uses highest-version-wins (seção 6.2)"
            .to_string(),
    );
    notes.push(
        "no [repositories] were generated — jvm-fast defaults to Maven Central; add \
         [repositories].default manually if the Gradle build used a different repository"
            .to_string(),
    );

    let mut seen = HashSet::new();
    let dependencies: Vec<(String, ImportedVersion)> = module
        .dependencies
        .iter()
        .filter(|dep| {
            dep.configuration == "compileClasspath" || dep.configuration == "runtimeClasspath"
        })
        .filter(|dep| seen.insert(dep.coordinate.clone()))
        .map(|dep| {
            (
                dep.coordinate.clone(),
                ImportedVersion::Explicit(dep.version.clone()),
            )
        })
        .collect();

    let mut dev_seen = HashSet::new();
    let dev_dependencies: Vec<(String, ImportedVersion)> = module
        .dependencies
        .iter()
        .filter(|dep| dep.configuration == "testCompileClasspath")
        .filter(|dep| !seen.contains(&dep.coordinate))
        .filter(|dep| dev_seen.insert(dep.coordinate.clone()))
        .map(|dep| {
            (
                dep.coordinate.clone(),
                ImportedVersion::Explicit(dep.version.clone()),
            )
        })
        .collect();

    let manifest = render_manifest(
        &module.name,
        &version,
        "lts",
        None,
        &dependencies,
        &dev_dependencies,
        &[],
        &[],
        &[],
    );

    std::fs::write(manifest_path, manifest).map_err(|source| GradleImportError::Io {
        path: manifest_path.to_path_buf(),
        source,
    })?;

    Ok(ImportReport { notes })
}
