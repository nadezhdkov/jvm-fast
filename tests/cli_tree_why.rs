use jvmfast::cli::{format_tree, format_why};
use jvmfast::domain::{Dependency, Module, VersionReq};
use jvmfast::pom::{ParsedPom, PomDependency, PomProvider};
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

    fn with_pom(
        mut self,
        coordinate: &str,
        version: &str,
        dependencies: Vec<(&str, &str)>,
    ) -> Self {
        self.poms.insert(
            (coordinate.to_string(), version.to_string()),
            ParsedPom {
                dependencies: dependencies
                    .into_iter()
                    .map(|(c, v)| PomDependency {
                        coordinate: c.to_string(),
                        version: v.to_string(),
                        scope: String::new(),
                    })
                    .collect(),
                managed_dependencies: Vec::new(),
            },
        );
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

fn dep(coordinate: &str, version: &str) -> Dependency {
    Dependency {
        coordinate: coordinate.to_string(),
        version_req: VersionReq::Explicit(version.to_string()),
    }
}

fn module(name: &str, deps: Vec<Dependency>) -> Module {
    Module {
        name: name.to_string(),
        root: PathBuf::from("."),
        declared_dependencies: deps,
        boms: Vec::new(),
        exclusions: HashMap::new(),
    }
}

/// core -> jackson-databind -> jackson-core
#[test]
fn format_tree_renders_nested_transitive_dependencies() {
    let modules = vec![module(
        "core",
        vec![dep("com.fasterxml.jackson.core:jackson-databind", "2.17.0")],
    )];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.fasterxml.jackson.core:jackson-databind",
            "2.17.0",
            vec![("com.fasterxml.jackson.core:jackson-core", "2.17.0")],
        )
        .with_pom("com.fasterxml.jackson.core:jackson-core", "2.17.0", vec![]);

    let output = resolve(&modules, &provider).expect("should resolve");
    let tree = format_tree(&output.graph, &output.module_roots, &modules);

    assert_eq!(
        tree,
        "core\n\
         └── com.fasterxml.jackson.core:jackson-databind:2.17.0\n\
         \u{20}\u{20}\u{20}\u{20}└── com.fasterxml.jackson.core:jackson-core:2.17.0\n"
    );
}

/// core -> b@1.0 -> d@1.0
/// core -> c@1.0 -> d@2.0
/// `why` deve reportar a versão vencedora e o motivo da mediação.
#[test]
fn format_why_reports_selected_version_and_mediation_reason() {
    let modules = vec![module(
        "core",
        vec![dep("com.example:b", "1.0"), dep("com.example:c", "1.0")],
    )];
    let provider = FixturePomProvider::new()
        .with_pom("com.example:b", "1.0", vec![("com.example:d", "1.0")])
        .with_pom("com.example:c", "1.0", vec![("com.example:d", "2.0")])
        .with_pom("com.example:d", "1.0", vec![])
        .with_pom("com.example:d", "2.0", vec![]);

    let output = resolve(&modules, &provider).expect("should resolve");
    let report = format_why(&output.graph, &output.module_roots, "com.example:d")
        .expect("coordinate should be in the resolved graph");

    assert!(report.starts_with("com.example:d:2.0\n"));
    assert!(report.contains("core\n"));
    assert!(report.contains("Resolution:"));
    assert!(report.contains("selected: com.example:d:2.0"));
    assert!(report.contains("higher version selected as tie-breaker"));
}

#[test]
fn format_why_returns_none_for_unknown_coordinate() {
    let modules = vec![module("core", vec![dep("com.example:a", "1.0")])];
    let provider = FixturePomProvider::new().with_pom("com.example:a", "1.0", vec![]);

    let output = resolve(&modules, &provider).expect("should resolve");
    let report = format_why(&output.graph, &output.module_roots, "com.example:missing");

    assert!(report.is_none());
}
