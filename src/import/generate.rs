/// Valor de uma entrada em `[dependencies]`/`[dev-dependencies]` — espelha
/// `manifest::dto::DependencyValue` (versão explícita, ou gerenciada por
/// BOM), mas do lado de *escrita* em vez de leitura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportedVersion {
    Explicit(String),
    BomManaged,
}

/// Monta o texto de um `project.toml` novo a partir dos dados já
/// resolvidos por `crate::import::import_pom` — função pura, sem I/O, para
/// que a formatação em si seja testável sem um arquivo `pom.xml` real.
#[allow(clippy::too_many_arguments)]
pub fn render_manifest(
    name: &str,
    version: &str,
    java_version: &str,
    source_encoding: Option<&str>,
    dependencies: &[(String, ImportedVersion)],
    dev_dependencies: &[(String, ImportedVersion)],
    boms: &[(String, String)],
    exclusions: &[(String, Vec<String>)],
    repositories: &[(String, String)],
) -> String {
    let mut out = String::new();

    out.push_str("[project]\n");
    out.push_str(&format!("name = {}\n", quote(name)));
    out.push_str(&format!("version = {}\n", quote(version)));
    out.push_str(&format!("java-version = {}\n", quote(java_version)));
    if let Some(encoding) = source_encoding {
        out.push_str(&format!("source-encoding = {}\n", quote(encoding)));
    }

    out.push_str("\n[dependencies]\n");
    for (coordinate, value) in dependencies {
        out.push_str(&format!(
            "{} = {}\n",
            quote(coordinate),
            render_value(value)
        ));
    }

    if !dev_dependencies.is_empty() {
        out.push_str("\n[dev-dependencies]\n");
        for (coordinate, value) in dev_dependencies {
            out.push_str(&format!(
                "{} = {}\n",
                quote(coordinate),
                render_value(value)
            ));
        }
    }

    if !boms.is_empty() {
        out.push_str("\n[boms]\n");
        for (coordinate, bom_version) in boms {
            out.push_str(&format!("{} = {}\n", quote(coordinate), quote(bom_version)));
        }
    }

    if !exclusions.is_empty() {
        out.push_str("\n[exclusions]\n");
        for (coordinate, excluded) in exclusions {
            let list = excluded
                .iter()
                .map(|e| quote(e))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("{} = [{}]\n", quote(coordinate), list));
        }
    }

    if !repositories.is_empty() {
        out.push_str("\n[repositories]\n");
        for (key, url) in repositories {
            out.push_str(&format!("{} = {}\n", quote(key), quote(url)));
        }
    }

    out
}

fn render_value(value: &ImportedVersion) -> String {
    match value {
        ImportedVersion::Explicit(v) => quote(v),
        ImportedVersion::BomManaged => "true".to_string(),
    }
}

fn quote(raw: &str) -> String {
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
