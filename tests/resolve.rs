use jvmfast::domain::{BomReference, Dependency, MediationReason, Module, VersionReq};
use jvmfast::pom::{ManagedDependencyEntry, ParsedPom, PomDependency, PomProvider};
use jvmfast::resolve::resolve;
use std::collections::HashMap;
use std::path::PathBuf;

struct FixturePomProvider {
    poms: HashMap<(String, String), ParsedPom>,
}

impl FixturePomProvider {
    fn new() -> Self {
        Self {
            poms: HashMap::new(),
        }
    }

    fn with_pom(mut self, coordinate: &str, version: &str, pom: ParsedPom) -> Self {
        self.poms
            .insert((coordinate.to_string(), version.to_string()), pom);
        self
    }
}

impl PomProvider for FixturePomProvider {
    fn fetch(
        &self,
        coordinate: &str,
        version: &str,
    ) -> Result<ParsedPom, Box<dyn std::error::Error + Send + Sync>> {
        self.poms
            .get(&(coordinate.to_string(), version.to_string()))
            .cloned()
            .ok_or_else(|| format!("no fixture POM for {coordinate}:{version}").into())
    }
}

fn plain_pom(dependencies: Vec<(&str, &str)>) -> ParsedPom {
    ParsedPom {
        dependencies: dependencies
            .into_iter()
            .map(|(coordinate, version)| PomDependency {
                coordinate: coordinate.to_string(),
                version: version.to_string(),
                scope: String::new(),
                ..Default::default()
            })
            .collect(),
        managed_dependencies: Vec::new(),
        ..Default::default()
    }
}

fn module(
    name: &str,
    deps: Vec<Dependency>,
    boms: Vec<BomReference>,
    exclusions: HashMap<String, Vec<String>>,
) -> Module {
    Module {
        name: name.to_string(),
        root: PathBuf::from("."),
        declared_dependencies: deps,
        boms,
        exclusions,
    }
}

fn dep_explicit(coordinate: &str, version: &str) -> Dependency {
    Dependency {
        coordinate: coordinate.to_string(),
        version_req: VersionReq::Explicit(version.to_string()),
    }
}

fn dep_bom_managed(coordinate: &str) -> Dependency {
    Dependency {
        coordinate: coordinate.to_string(),
        version_req: VersionReq::BomManaged,
    }
}

#[test]
fn resolves_bom_managed_dependency_end_to_end() {
    let modules = vec![module(
        "core",
        vec![dep_bom_managed(
            "com.fasterxml.jackson.core:jackson-databind",
        )],
        vec![BomReference {
            coordinate: "com.fasterxml.jackson:jackson-bom".to_string(),
            version: "2.17.0".to_string(),
        }],
        HashMap::new(),
    )];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.fasterxml.jackson:jackson-bom",
            "2.17.0",
            ParsedPom {
                dependencies: Vec::new(),
                managed_dependencies: vec![ManagedDependencyEntry {
                    coordinate: "com.fasterxml.jackson.core:jackson-databind".to_string(),
                    version: "2.17.0".to_string(),
                    is_bom_import: false,
                }],
                ..Default::default()
            },
        )
        .with_pom(
            "com.fasterxml.jackson.core:jackson-databind",
            "2.17.0",
            plain_pom(vec![]),
        );

    let output = resolve(&modules, &provider).expect("should resolve");

    let node = output
        .graph
        .nodes
        .values()
        .find(|n| n.coordinate == "com.fasterxml.jackson.core:jackson-databind")
        .expect("should have resolved the BOM-managed dependency");
    assert_eq!(node.selected, "2.17.0");
    assert_eq!(output.module_roots.len(), 1);
}

#[test]
fn excluded_transitive_never_reaches_the_resolved_graph() {
    let mut exclusions = HashMap::new();
    exclusions.insert(
        "com.example:parent".to_string(),
        vec!["com.example:unwanted".to_string()],
    );
    let modules = vec![module(
        "core",
        vec![dep_explicit("com.example:parent", "1.0")],
        Vec::new(),
        exclusions,
    )];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:parent",
            "1.0",
            plain_pom(vec![("com.example:unwanted", "1.0")]),
        )
        .with_pom("com.example:unwanted", "1.0", plain_pom(vec![]));

    let output = resolve(&modules, &provider).expect("should resolve");

    assert!(output
        .graph
        .nodes
        .values()
        .all(|n| n.coordinate != "com.example:unwanted"));
}

#[test]
fn mediates_conflicting_versions_across_the_full_pipeline() {
    let modules = vec![module(
        "core",
        vec![
            dep_explicit("com.example:b", "1.0"),
            dep_explicit("com.example:c", "1.0"),
        ],
        Vec::new(),
        HashMap::new(),
    )];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:b",
            "1.0",
            plain_pom(vec![("com.example:d", "1.0")]),
        )
        .with_pom(
            "com.example:c",
            "1.0",
            plain_pom(vec![("com.example:d", "2.0")]),
        )
        .with_pom("com.example:d", "1.0", plain_pom(vec![]))
        .with_pom("com.example:d", "2.0", plain_pom(vec![]));

    let output = resolve(&modules, &provider).expect("should resolve");

    let node_d = output
        .graph
        .nodes
        .values()
        .find(|n| n.coordinate == "com.example:d")
        .unwrap();
    assert_eq!(node_d.selected, "2.0");
    assert_eq!(
        node_d.mediation_reason,
        MediationReason::HigherVersionWins {
            rejected: vec!["1.0".to_string()]
        }
    );
}

#[test]
fn missing_bom_managed_version_is_a_typed_error() {
    let modules = vec![module(
        "core",
        vec![dep_bom_managed("com.example:undeclared")],
        Vec::new(),
        HashMap::new(),
    )];
    let provider = FixturePomProvider::new();

    let result = resolve(&modules, &provider);

    assert!(result.is_err());
}
