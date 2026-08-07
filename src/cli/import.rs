use super::error::CliError;
use crate::cli::context::cache_root;
use crate::gradleimport::import_gradle as import_gradle_impl;
use crate::import::{import_pom as import_pom_impl, ImportReport};
use std::path::Path;

/// Wires `jvmfast import-pom` (docs/architecture.md seção 10): reads
/// `pom` (defaults to `pom.xml` at the project root) and writes
/// `project.toml` at the root — never touches `pom.xml`, and never
/// overwrites an existing `project.toml` (`ImportError::ManifestAlreadyExists`).
pub fn run(root: &Path, pom: Option<String>) -> Result<String, CliError> {
    let pom_path = match pom {
        Some(path) => Path::new(&path).to_path_buf(),
        None => root.join("pom.xml"),
    };
    let manifest_path = root.join("project.toml");

    let report = import_pom_impl(&pom_path, &manifest_path)?;
    Ok(format_summary(
        &format!("project.toml written from {}", pom_path.display()),
        &report,
    ))
}

/// Wires `jvmfast import-gradle` (docs/architecture.md seção 10): reads
/// `project` (defaults to the current project root) through the Gradle
/// Tooling API bridge (`crate::gradleimport`) and writes `project.toml` at
/// the root — never touches the source Gradle build files, and never
/// overwrites an existing `project.toml`
/// (`GradleImportError::ManifestAlreadyExists`).
pub fn run_gradle(root: &Path, project: Option<String>) -> Result<String, CliError> {
    let project_dir = match project {
        Some(path) => Path::new(&path).to_path_buf(),
        None => root.to_path_buf(),
    };
    let manifest_path = root.join("project.toml");

    let report = import_gradle_impl(&project_dir, &manifest_path, &cache_root())?;
    Ok(format_summary(
        &format!("project.toml written from {}", project_dir.display()),
        &report,
    ))
}

fn format_summary(header: &str, report: &ImportReport) -> String {
    let mut summary = header.to_string();
    if report.notes.is_empty() {
        return summary;
    }
    summary.push_str(&format!(
        "\n{} item(s) need manual attention:",
        report.notes.len()
    ));
    for note in &report.notes {
        summary.push_str(&format!("\n  - {note}"));
    }
    summary
}
