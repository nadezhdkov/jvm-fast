use super::error::PomParseError;
use super::{ManagedDependencyEntry, ParsedPom, PomDependency};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Reader;
use std::collections::HashMap;

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
    exclusions: Vec<String>,
}

impl DependencyAccumulator {
    fn coordinate(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }
}

#[derive(Default)]
struct ExclusionAccumulator {
    group_id: String,
    artifact_id: String,
}

impl ExclusionAccumulator {
    fn coordinate(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }
}

#[derive(Default)]
struct RepositoryAccumulator {
    id: String,
    url: String,
}

/// O que o texto lido a seguir deve alimentar — recalculado a cada
/// `on_open` a partir da pilha de tags atual, nunca guardado entre chamadas
/// (evita que estado de um ramo vaze para outro irmão na árvore).
enum TextTarget {
    ProjectArtifactId,
    ProjectVersion,
    Property(String),
    DepField(String),
    ExclusionField(String),
    RepoField(String),
    None,
}

/// Parseia um `pom.xml` para as seções `<dependencies>` (diretas),
/// `<dependencyManagement>/<dependencies>` (gerenciadas), `<properties>`,
/// `<repositories>`, exclusões por dependência, e presença de
/// `<profiles>`/`<build><plugins>` (usado por `crate::import`, seção 10).
/// Não interpola `${propriedade}` durante o parse (isso é
/// `crate::import`, quando aplicável — a resolução normal nunca interpola,
/// ver `ParsedPom::properties`), nem segue herança de POM pai
/// (`<parent>`) — escopo deliberado desta passada, não um bug escondido.
pub fn parse_pom_xml(xml: &str) -> Result<ParsedPom, PomParseError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<String> = Vec::new();
    // seção ativa + profundidade da tag <dependencies> que a abriu
    let mut section: Option<(Section, usize)> = None;
    // profundidade da tag <dependency> em construção, se houver
    let mut in_dependency: Option<usize> = None;
    // profundidade da tag <exclusion> em construção, se houver
    let mut in_exclusion: Option<usize> = None;
    // profundidade da tag <repository> em construção, se houver
    let mut in_repository: Option<usize> = None;
    let mut current_target = TextTarget::None;
    let mut accumulator = DependencyAccumulator::default();
    let mut exclusion_accumulator = ExclusionAccumulator::default();
    let mut repository_accumulator = RepositoryAccumulator::default();

    let mut dependencies = Vec::new();
    let mut managed_dependencies = Vec::new();
    let mut properties: HashMap<String, String> = HashMap::new();
    let mut repositories = Vec::new();
    let mut project_artifact_id = String::new();
    let mut project_version = String::new();
    let mut has_profiles = false;
    let mut has_plugins = false;

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Start(e) => {
                on_open(
                    &e,
                    &mut stack,
                    &mut section,
                    &mut in_dependency,
                    &mut in_exclusion,
                    &mut in_repository,
                    &mut current_target,
                    &mut accumulator,
                    &mut has_profiles,
                    &mut has_plugins,
                )?;
            }
            Event::Empty(e) => {
                on_open(
                    &e,
                    &mut stack,
                    &mut section,
                    &mut in_dependency,
                    &mut in_exclusion,
                    &mut in_repository,
                    &mut current_target,
                    &mut accumulator,
                    &mut has_profiles,
                    &mut has_plugins,
                )?;
                on_close(ClosingContext {
                    name: local_name(e.name())?,
                    stack: &mut stack,
                    section: &mut section,
                    in_dependency: &mut in_dependency,
                    in_exclusion: &mut in_exclusion,
                    in_repository: &mut in_repository,
                    current_target: &mut current_target,
                    accumulator: &mut accumulator,
                    exclusion_accumulator: &mut exclusion_accumulator,
                    repository_accumulator: &mut repository_accumulator,
                    dependencies: &mut dependencies,
                    managed_dependencies: &mut managed_dependencies,
                    repositories: &mut repositories,
                });
            }
            Event::Text(text) => {
                let raw = text.decode().map_err(quick_xml::Error::from)?;
                let decoded = quick_xml::escape::unescape(&raw).map_err(quick_xml::Error::from)?;
                match &current_target {
                    TextTarget::ProjectArtifactId => project_artifact_id.push_str(&decoded),
                    TextTarget::ProjectVersion => project_version.push_str(&decoded),
                    TextTarget::Property(key) => {
                        properties
                            .entry(key.clone())
                            .or_default()
                            .push_str(&decoded);
                    }
                    TextTarget::DepField(field) => match field.as_str() {
                        "groupId" => accumulator.group_id.push_str(&decoded),
                        "artifactId" => accumulator.artifact_id.push_str(&decoded),
                        "version" => accumulator.version.push_str(&decoded),
                        "type" => accumulator.dep_type.push_str(&decoded),
                        "scope" => accumulator.scope.push_str(&decoded),
                        _ => {}
                    },
                    TextTarget::ExclusionField(field) => match field.as_str() {
                        "groupId" => exclusion_accumulator.group_id.push_str(&decoded),
                        "artifactId" => exclusion_accumulator.artifact_id.push_str(&decoded),
                        _ => {}
                    },
                    TextTarget::RepoField(field) => match field.as_str() {
                        "id" => repository_accumulator.id.push_str(&decoded),
                        "url" => repository_accumulator.url.push_str(&decoded),
                        _ => {}
                    },
                    TextTarget::None => {}
                }
            }
            Event::End(e) => {
                let name = local_name(e.name())?;
                on_close(ClosingContext {
                    name,
                    stack: &mut stack,
                    section: &mut section,
                    in_dependency: &mut in_dependency,
                    in_exclusion: &mut in_exclusion,
                    in_repository: &mut in_repository,
                    current_target: &mut current_target,
                    accumulator: &mut accumulator,
                    exclusion_accumulator: &mut exclusion_accumulator,
                    repository_accumulator: &mut repository_accumulator,
                    dependencies: &mut dependencies,
                    managed_dependencies: &mut managed_dependencies,
                    repositories: &mut repositories,
                });
            }
            _ => {}
        }
    }

    Ok(ParsedPom {
        dependencies,
        managed_dependencies,
        project_artifact_id,
        project_version,
        properties,
        repositories,
        has_profiles,
        has_plugins,
    })
}

#[allow(clippy::too_many_arguments)]
fn on_open(
    e: &BytesStart,
    stack: &mut Vec<String>,
    section: &mut Option<(Section, usize)>,
    in_dependency: &mut Option<usize>,
    in_exclusion: &mut Option<usize>,
    in_repository: &mut Option<usize>,
    current_target: &mut TextTarget,
    accumulator: &mut DependencyAccumulator,
    has_profiles: &mut bool,
    has_plugins: &mut bool,
) -> Result<(), PomParseError> {
    let name = local_name(e.name())?;
    stack.push(name.clone());

    if stack_matches(stack, &["project", "profiles"]) {
        *has_profiles = true;
    }
    if stack_matches(stack, &["project", "build", "plugins"]) {
        *has_plugins = true;
    }

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

    if let Some(dep_depth) = in_dependency {
        if in_exclusion.is_none()
            && name == "exclusion"
            && stack.len() == *dep_depth + 2
            && stack.get(*dep_depth) == Some(&"exclusions".to_string())
        {
            *in_exclusion = Some(stack.len());
        }
    }

    if in_repository.is_none()
        && name == "repository"
        && stack_matches(&stack[..stack.len() - 1], &["project", "repositories"])
    {
        *in_repository = Some(stack.len());
    }

    *current_target = text_target(
        stack,
        *section,
        *in_dependency,
        *in_exclusion,
        *in_repository,
        &name,
    );

    Ok(())
}

fn text_target(
    stack: &[String],
    section: Option<(Section, usize)>,
    in_dependency: Option<usize>,
    in_exclusion: Option<usize>,
    in_repository: Option<usize>,
    name: &str,
) -> TextTarget {
    if let Some(excl_depth) = in_exclusion {
        if stack.len() == excl_depth + 1 && matches!(name, "groupId" | "artifactId") {
            return TextTarget::ExclusionField(name.to_string());
        }
        return TextTarget::None;
    }

    if let Some(repo_depth) = in_repository {
        if stack.len() == repo_depth + 1 && matches!(name, "id" | "url") {
            return TextTarget::RepoField(name.to_string());
        }
        return TextTarget::None;
    }

    if let Some(dep_depth) = in_dependency {
        if stack.len() == dep_depth + 1
            && matches!(
                name,
                "groupId" | "artifactId" | "version" | "type" | "scope"
            )
        {
            return TextTarget::DepField(name.to_string());
        }
        return TextTarget::None;
    }

    if section.is_none() {
        if stack == ["project", "artifactId"] {
            return TextTarget::ProjectArtifactId;
        }
        if stack == ["project", "version"] {
            return TextTarget::ProjectVersion;
        }
        if stack.len() == 3 && stack[0] == "project" && stack[1] == "properties" {
            return TextTarget::Property(stack[2].clone());
        }
    }

    TextTarget::None
}

struct ClosingContext<'a> {
    name: String,
    stack: &'a mut Vec<String>,
    section: &'a mut Option<(Section, usize)>,
    in_dependency: &'a mut Option<usize>,
    in_exclusion: &'a mut Option<usize>,
    in_repository: &'a mut Option<usize>,
    current_target: &'a mut TextTarget,
    accumulator: &'a mut DependencyAccumulator,
    exclusion_accumulator: &'a mut ExclusionAccumulator,
    repository_accumulator: &'a mut RepositoryAccumulator,
    dependencies: &'a mut Vec<PomDependency>,
    managed_dependencies: &'a mut Vec<ManagedDependencyEntry>,
    repositories: &'a mut Vec<(String, String)>,
}

fn on_close(ctx: ClosingContext) {
    let ClosingContext {
        name,
        stack,
        section,
        in_dependency,
        in_exclusion,
        in_repository,
        current_target,
        accumulator,
        exclusion_accumulator,
        repository_accumulator,
        dependencies,
        managed_dependencies,
        repositories,
    } = ctx;

    if let Some(excl_depth) = *in_exclusion {
        if stack.len() == excl_depth && name == "exclusion" {
            accumulator
                .exclusions
                .push(exclusion_accumulator.coordinate());
            *exclusion_accumulator = ExclusionAccumulator::default();
            *in_exclusion = None;
        }
    }

    if let Some(repo_depth) = *in_repository {
        if stack.len() == repo_depth && name == "repository" {
            repositories.push((
                repository_accumulator.id.clone(),
                repository_accumulator.url.clone(),
            ));
            *repository_accumulator = RepositoryAccumulator::default();
            *in_repository = None;
        }
    }

    if let Some(dep_depth) = *in_dependency {
        if stack.len() == dep_depth && name == "dependency" {
            match section {
                Some((Section::Plain, _)) => dependencies.push(PomDependency {
                    coordinate: accumulator.coordinate(),
                    version: accumulator.version.clone(),
                    scope: accumulator.scope.clone(),
                    exclusions: accumulator.exclusions.clone(),
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

    *current_target = TextTarget::None;
    stack.pop();
}

fn stack_matches(stack: &[String], expected: &[&str]) -> bool {
    stack.len() == expected.len() && stack.iter().zip(expected).all(|(a, b)| a == b)
}

fn local_name(name: QName) -> Result<String, PomParseError> {
    Ok(std::str::from_utf8(name.local_name().as_ref())?.to_string())
}
