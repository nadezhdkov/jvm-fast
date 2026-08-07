use jvmfast::domain::{Dependency, MediationReason, Module, VersionReq};
use jvmfast::graph::build_graph;
use jvmfast::mediation::mediate;
use jvmfast::pom::{ParsedPom, PomDependency, PomProvider};
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
        dependencies: Vec<PomDependency>,
    ) -> Self {
        self.poms.insert(
            (coordinate.to_string(), version.to_string()),
            ParsedPom {
                dependencies,
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

fn pom_dep(coordinate: &str, version: &str) -> PomDependency {
    PomDependency {
        coordinate: coordinate.to_string(),
        version: version.to_string(),
        scope: String::new(),
    }
}

fn dep_explicit(coordinate: &str, version: &str) -> Dependency {
    Dependency {
        coordinate: coordinate.to_string(),
        version_req: VersionReq::Explicit(version.to_string()),
    }
}

/// core -> b -> d@1.0
/// core -> c -> d@2.0
/// Mesma profundidade (2) para as duas requisições de "d", versões
/// diferentes — o caso clássico de diamond dependency da seção 13.1.
#[test]
fn diamond_dependency_resolves_via_higher_version_at_same_depth() {
    let modules = vec![Module {
        name: "core".to_string(),
        root: PathBuf::from("."),
        declared_dependencies: vec![
            dep_explicit("com.example:b", "1.0"),
            dep_explicit("com.example:c", "1.0"),
        ],
        boms: Vec::new(),
        exclusions: HashMap::new(),
    }];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:b",
            "1.0",
            vec![pom_dep("com.example:d", "1.0")],
        )
        .with_pom(
            "com.example:c",
            "1.0",
            vec![pom_dep("com.example:d", "2.0")],
        )
        .with_pom("com.example:d", "1.0", vec![])
        .with_pom("com.example:d", "2.0", vec![]);

    let candidate_graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");
    let result = mediate(candidate_graph);

    let node_d = result
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

    let node_b = result
        .graph
        .nodes
        .values()
        .find(|n| n.coordinate == "com.example:b")
        .unwrap();
    assert_eq!(node_b.mediation_reason, MediationReason::SingleRequest);
}

/// Direta @1.0 (profundidade 1) vence transitiva @2.0 (profundidade 2),
/// mesmo a transitiva tendo versão numericamente maior — seção 13.1,
/// "Depth wins over version".
#[test]
fn direct_dependency_wins_over_deeper_transitive_with_higher_version() {
    let modules = vec![Module {
        name: "core".to_string(),
        root: PathBuf::from("."),
        declared_dependencies: vec![
            dep_explicit("com.exemplo:commons", "1.8"),
            dep_explicit("com.example:library-a", "1.0"),
        ],
        boms: Vec::new(),
        exclusions: HashMap::new(),
    }];
    let provider = FixturePomProvider::new()
        .with_pom("com.exemplo:commons", "1.8", vec![])
        .with_pom(
            "com.example:library-a",
            "1.0",
            vec![pom_dep("com.exemplo:commons", "2.0")],
        )
        .with_pom("com.exemplo:commons", "2.0", vec![]);

    let candidate_graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");
    let result = mediate(candidate_graph);

    let node_commons = result
        .graph
        .nodes
        .values()
        .find(|n| n.coordinate == "com.exemplo:commons")
        .unwrap();
    assert_eq!(node_commons.selected, "1.8");
    assert_eq!(
        node_commons.mediation_reason,
        MediationReason::NearestDepthWins {
            rejected: vec!["2.0".to_string()]
        }
    );
}
