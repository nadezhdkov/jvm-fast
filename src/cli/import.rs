use super::error::CliError;
use crate::import::import_pom as import_pom_impl;
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

    let mut summary = format!("project.toml written from {}", pom_path.display());
    if report.notes.is_empty() {
        return Ok(summary);
    }
    summary.push_str(&format!(
        "\n{} item(s) need manual attention:",
        report.notes.len()
    ));
    for note in &report.notes {
        summary.push_str(&format!("\n  - {note}"));
    }
    Ok(summary)
}
