use super::error::CliError;
use crate::init::init_project;
use std::path::Path;

/// Wires `jvmfast init` (docs/architecture.md seção 9.2): writes a minimal
/// `project.toml` plus `src/main/java`/`src/test/java` at `root`. See
/// `crate::init::init_project` for the full behavior (refuses to run over
/// an existing `project.toml`/`pom.xml`, non-interactive `--name`/
/// `--java-version` defaulting).
pub fn run(
    root: &Path,
    name: Option<String>,
    java_version: Option<String>,
) -> Result<String, CliError> {
    let report = init_project(root, name.as_deref(), java_version.as_deref())?;
    Ok(format_summary(root, &report))
}

fn format_summary(root: &Path, report: &crate::init::InitReport) -> String {
    let mut summary = format!("project.toml written at {}", root.display());
    if report.notes.is_empty() {
        return summary;
    }
    for note in &report.notes {
        summary.push_str(&format!("\n  - {note}"));
    }
    summary
}
