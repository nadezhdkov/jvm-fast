use super::error::PomParseError;
use super::{ManagedDependencyEntry, ParsedPom, PomDependency};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Reader;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Plain,
    Managed,
}

#[derive(Default)]
struct DependencyAccumulator {
    group_id: String,
    artifact_id: String,
    version: String,
    dep_type: String,
    scope: String,
}

impl DependencyAccumulator {
    fn coordinate(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }
}

/// Parseia um `pom.xml` para as seções `<dependencies>` (diretas) e
/// `<dependencyManagement>/<dependencies>` (gerenciadas). Não interpola
/// `${propriedade}`, não segue herança de POM pai (`<parent>`) — escopo
/// deliberado desta passada, não um bug escondido.
pub fn parse_pom_xml(xml: &str) -> Result<ParsedPom, PomParseError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<String> = Vec::new();
    // seção ativa + profundidade da tag <dependencies> que a abriu
    let mut section: Option<(Section, usize)> = None;
    // profundidade da tag <dependency> em construção, se houver
    let mut in_dependency: Option<usize> = None;
    let mut current_field: Option<String> = None;
    let mut accumulator = DependencyAccumulator::default();

    let mut dependencies = Vec::new();
    let mut managed_dependencies = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Start(e) => {
                on_open(
                    &e,
                    &mut stack,
                    &mut section,
                    &mut in_dependency,
                    &mut current_field,
                    &mut accumulator,
                )?;
            }
            Event::Empty(e) => {
                on_open(
                    &e,
                    &mut stack,
                    &mut section,
                    &mut in_dependency,
                    &mut current_field,
                    &mut accumulator,
                )?;
                on_close(
                    local_name(e.name())?,
                    &mut stack,
                    &mut section,
                    &mut in_dependency,
                    &mut current_field,
                    &accumulator,
                    &mut dependencies,
                    &mut managed_dependencies,
                );
            }
            Event::Text(text) => {
                if let Some(field) = &current_field {
                    let raw = text.decode().map_err(quick_xml::Error::from)?;
                    let decoded =
                        quick_xml::escape::unescape(&raw).map_err(quick_xml::Error::from)?;
                    match field.as_str() {
                        "groupId" => accumulator.group_id.push_str(&decoded),
                        "artifactId" => accumulator.artifact_id.push_str(&decoded),
                        "version" => accumulator.version.push_str(&decoded),
                        "type" => accumulator.dep_type.push_str(&decoded),
                        "scope" => accumulator.scope.push_str(&decoded),
                        _ => {}
                    }
                }
            }
            Event::End(e) => {
                let name = local_name(e.name())?;
                on_close(
                    name,
                    &mut stack,
                    &mut section,
                    &mut in_dependency,
                    &mut current_field,
                    &accumulator,
                    &mut dependencies,
                    &mut managed_dependencies,
                );
            }
            _ => {}
        }
    }

    Ok(ParsedPom {
        dependencies,
        managed_dependencies,
    })
}

fn on_open(
    e: &BytesStart,
    stack: &mut Vec<String>,
    section: &mut Option<(Section, usize)>,
    in_dependency: &mut Option<usize>,
    current_field: &mut Option<String>,
    accumulator: &mut DependencyAccumulator,
) -> Result<(), PomParseError> {
    let name = local_name(e.name())?;
    stack.push(name.clone());

    if section.is_none() && name == "dependencies" {
        if stack_matches(stack, &["project", "dependencies"]) {
            *section = Some((Section::Plain, stack.len()));
        } else if stack_matches(stack, &["project", "dependencyManagement", "dependencies"]) {
            *section = Some((Section::Managed, stack.len()));
        }
    }

    if let Some((_, deps_depth)) = section {
        if in_dependency.is_none() && name == "dependency" && stack.len() == *deps_depth + 1 {
            *in_dependency = Some(stack.len());
            *accumulator = DependencyAccumulator::default();
        }
    }

    *current_field = match in_dependency {
        Some(dep_depth) if stack.len() == *dep_depth + 1 => match name.as_str() {
            "groupId" | "artifactId" | "version" | "type" | "scope" => Some(name),
            _ => None,
        },
        _ => None,
    };

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn on_close(
    name: String,
    stack: &mut Vec<String>,
    section: &mut Option<(Section, usize)>,
    in_dependency: &mut Option<usize>,
    current_field: &mut Option<String>,
    accumulator: &DependencyAccumulator,
    dependencies: &mut Vec<PomDependency>,
    managed_dependencies: &mut Vec<ManagedDependencyEntry>,
) {
    if let Some(dep_depth) = *in_dependency {
        if stack.len() == dep_depth && name == "dependency" {
            match section {
                Some((Section::Plain, _)) => dependencies.push(PomDependency {
                    coordinate: accumulator.coordinate(),
                    version: accumulator.version.clone(),
                    scope: accumulator.scope.clone(),
                }),
                Some((Section::Managed, _)) => managed_dependencies.push(ManagedDependencyEntry {
                    coordinate: accumulator.coordinate(),
                    version: accumulator.version.clone(),
                    is_bom_import: accumulator.dep_type == "pom" && accumulator.scope == "import",
                }),
                None => {}
            }
            *in_dependency = None;
        }
    }

    if let Some((_, deps_depth)) = *section {
        if stack.len() == deps_depth && name == "dependencies" {
            *section = None;
        }
    }

    *current_field = None;
    stack.pop();
}

fn stack_matches(stack: &[String], expected: &[&str]) -> bool {
    stack.len() == expected.len() && stack.iter().zip(expected).all(|(a, b)| a == b)
}

fn local_name(name: QName) -> Result<String, PomParseError> {
    Ok(std::str::from_utf8(name.local_name().as_ref())?.to_string())
}
