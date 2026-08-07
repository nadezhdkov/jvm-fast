use jvmfast::domain::{Dependency, EdgeKind, Module, VersionReq};
use jvmfast::exclusion::merge_exclusions;
use jvmfast::graph::{build_graph, GraphError};
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
                ..Default::default()
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
        ..Default::default()
    }
}

fn pom_dep_with_scope(coordinate: &str, version: &str, scope: &str) -> PomDependency {
    PomDependency {
        coordinate: coordinate.to_string(),
        version: version.to_string(),
        scope: scope.to_string(),
        ..Default::default()
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

fn module(name: &str, dependencies: Vec<Dependency>) -> Module {
    Module {
        name: name.to_string(),
        root: PathBuf::from("."),
        declared_dependencies: dependencies,
        boms: Vec::new(),
        exclusions: HashMap::new(),
        workspace_dependencies: Vec::new(),
    }
}

fn module_with_exclusions(
    name: &str,
    dependencies: Vec<Dependency>,
    exclusions: HashMap<String, Vec<String>>,
) -> Module {
    Module {
        name: name.to_string(),
        root: PathBuf::from("."),
        declared_dependencies: dependencies,
        boms: Vec::new(),
        exclusions,
        workspace_dependencies: Vec::new(),
    }
}

fn module_with_workspace_dependencies(
    name: &str,
    dependencies: Vec<Dependency>,
    workspace_dependencies: Vec<String>,
) -> Module {
    Module {
        name: name.to_string(),
        root: PathBuf::from("."),
        declared_dependencies: dependencies,
        boms: Vec::new(),
        exclusions: HashMap::new(),
        workspace_dependencies,
    }
}

#[test]
fn direct_dependency_becomes_depth_one_candidate() {
    let modules = vec![module(
        "core",
        vec![dep_explicit("org.slf4j:slf4j-api", "2.0.13")],
    )];
    let provider = FixturePomProvider::new().with_pom("org.slf4j:slf4j-api", "2.0.13", vec![]);

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 1);
    let node = graph.candidates.values().next().unwrap();
    assert_eq!(node.coordinate, "org.slf4j:slf4j-api");
    assert_eq!(node.requests.len(), 1);
    assert_eq!(node.requests[0].depth, 1);
    assert_eq!(node.requests[0].origin_module, "core");
    assert_eq!(node.requests[0].version, "2.0.13");

    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].kind, EdgeKind::ModuleDeclared);
    assert_eq!(graph.edges[0].from, graph.module_roots["core"]);
}

#[test]
fn transitive_dependency_gets_depth_two_and_external_edge() {
    let modules = vec![module("core", vec![dep_explicit("com.example:a", "1.0")])];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:a",
            "1.0",
            vec![pom_dep("com.example:b", "2.0")],
        )
        .with_pom("com.example:b", "2.0", vec![]);

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 2);
    let (&node_id_b, node_b) = graph
        .candidates
        .iter()
        .find(|(_, n)| n.coordinate == "com.example:b")
        .unwrap();
    assert_eq!(node_b.requests[0].depth, 2);
    assert_eq!(node_b.requests[0].origin_module, "core");

    let edge_to_b = graph.edges.iter().find(|e| e.to == node_id_b).unwrap();
    assert_eq!(edge_to_b.kind, EdgeKind::External);
}

#[test]
fn bom_managed_dependency_uses_bom_table_version() {
    let modules = vec![module(
        "core",
        vec![dep_bom_managed(
            "com.fasterxml.jackson.core:jackson-databind",
        )],
    )];
    let provider = FixturePomProvider::new().with_pom(
        "com.fasterxml.jackson.core:jackson-databind",
        "2.17.0",
        vec![],
    );
    let bom_table = HashMap::from([(
        "com.fasterxml.jackson.core:jackson-databind".to_string(),
        "2.17.0".to_string(),
    )]);

    let graph =
        build_graph(&modules, &bom_table, &HashMap::new(), &provider).expect("should build");

    let node = graph.candidates.values().next().unwrap();
    assert_eq!(node.requests[0].version, "2.17.0");
}

#[test]
fn bom_managed_dependency_without_bom_entry_is_a_typed_error() {
    let modules = vec![module(
        "core",
        vec![dep_bom_managed("com.example:unmanaged")],
    )];
    let provider = FixturePomProvider::new();

    let result = build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider);

    assert!(
        matches!(result, Err(GraphError::MissingBomManagedVersion(coord)) if coord == "com.example:unmanaged")
    );
}

#[test]
fn unresolved_version_range_is_a_typed_error() {
    let modules = vec![module(
        "core",
        vec![dep_explicit(
            "com.fasterxml.jackson.core:jackson-databind",
            "^2.17.0",
        )],
    )];
    let provider = FixturePomProvider::new();

    let result = build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider);

    assert!(matches!(
        result,
        Err(GraphError::UnresolvedVersionRange { .. })
    ));
}

#[test]
fn excluded_transitive_never_becomes_a_candidate() {
    let modules = vec![module_with_exclusions(
        "core",
        vec![dep_explicit(
            "org.apache.httpcomponents:httpclient",
            "4.5.14",
        )],
        HashMap::from([(
            "org.apache.httpcomponents:httpclient".to_string(),
            vec!["commons-logging:commons-logging".to_string()],
        )]),
    )];
    let provider = FixturePomProvider::new().with_pom(
        "org.apache.httpcomponents:httpclient",
        "4.5.14",
        vec![pom_dep("commons-logging:commons-logging", "1.2")],
    );
    let exclusions = merge_exclusions(&modules);

    let graph =
        build_graph(&modules, &HashMap::new(), &exclusions, &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 1);
    assert!(graph
        .candidates
        .values()
        .all(|n| n.coordinate != "commons-logging:commons-logging"));
}

#[test]
fn multiple_modules_requesting_same_coordinate_produce_two_requests() {
    let modules = vec![
        module("core", vec![dep_explicit("com.example:shared", "1.0")]),
        module("api", vec![dep_explicit("com.example:shared", "1.0")]),
    ];
    let provider = FixturePomProvider::new().with_pom("com.example:shared", "1.0", vec![]);

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 1);
    let node = graph.candidates.values().next().unwrap();
    assert_eq!(node.requests.len(), 2);
    let origins: Vec<&str> = node
        .requests
        .iter()
        .map(|r| r.origin_module.as_str())
        .collect();
    assert!(origins.contains(&"core"));
    assert!(origins.contains(&"api"));
}

#[test]
fn circular_dependency_does_not_infinite_loop() {
    let modules = vec![module("core", vec![dep_explicit("com.example:a", "1.0")])];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:a",
            "1.0",
            vec![pom_dep("com.example:b", "1.0")],
        )
        .with_pom(
            "com.example:b",
            "1.0",
            vec![pom_dep("com.example:a", "1.0")],
        );

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 2);
    assert_eq!(graph.edges.len(), 3);
    let node_a = graph
        .candidates
        .values()
        .find(|n| n.coordinate == "com.example:a")
        .unwrap();
    assert_eq!(node_a.requests.len(), 2);
}

#[test]
fn test_scoped_transitive_never_becomes_a_candidate() {
    let modules = vec![module("core", vec![dep_explicit("com.example:a", "1.0")])];
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:a",
            "1.0",
            vec![pom_dep_with_scope("com.example:b", "2.0", "test")],
        )
        .with_pom("com.example:b", "2.0", vec![]);

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 1);
    assert!(graph
        .candidates
        .values()
        .all(|n| n.coordinate != "com.example:b"));
}

#[test]
fn provided_scoped_transitive_never_becomes_a_candidate() {
    let modules = vec![module("core", vec![dep_explicit("com.example:a", "1.0")])];
    let provider = FixturePomProvider::new().with_pom(
        "com.example:a",
        "1.0",
        vec![pom_dep_with_scope("com.example:b", "2.0", "provided")],
    );

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 1);
}

#[test]
fn system_scoped_transitive_never_becomes_a_candidate() {
    let modules = vec![module("core", vec![dep_explicit("com.example:a", "1.0")])];
    let provider = FixturePomProvider::new().with_pom(
        "com.example:a",
        "1.0",
        vec![pom_dep_with_scope("com.example:b", "2.0", "system")],
    );

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.candidates.len(), 1);
}

#[test]
fn compile_and_runtime_and_default_scoped_transitives_still_propagate() {
    for scope in ["compile", "runtime", ""] {
        let modules = vec![module("core", vec![dep_explicit("com.example:a", "1.0")])];
        let provider = FixturePomProvider::new()
            .with_pom(
                "com.example:a",
                "1.0",
                vec![pom_dep_with_scope("com.example:b", "2.0", scope)],
            )
            .with_pom("com.example:b", "2.0", vec![]);

        let graph = build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider)
            .unwrap_or_else(|_| panic!("should build with scope {scope:?}"));

        assert!(
            graph
                .candidates
                .values()
                .any(|n| n.coordinate == "com.example:b"),
            "scope {scope:?} should propagate transitively"
        );
    }
}

#[test]
fn fetch_failure_is_wrapped_as_typed_error() {
    let modules = vec![module(
        "core",
        vec![dep_explicit("com.example:missing", "1.0")],
    )];
    let provider = FixturePomProvider::new();

    let result = build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider);

    assert!(matches!(result, Err(GraphError::Fetch { .. })));
}

#[test]
fn workspace_dependency_becomes_a_workspace_module_edge_between_module_roots() {
    let modules = vec![
        module_with_workspace_dependencies("api", vec![], vec!["core".to_string()]),
        module("core", vec![]),
    ];
    let provider = FixturePomProvider::new();

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    let api_root = graph.module_roots["api"];
    let core_root = graph.module_roots["core"];
    assert!(graph.edges.iter().any(|edge| edge.from == api_root
        && edge.to == core_root
        && edge.kind == EdgeKind::WorkspaceModule));
    // A workspace-module edge never introduces a resolution candidate —
    // there's no version to mediate for a sibling module.
    assert!(graph.candidates.is_empty());
}

#[test]
fn workspace_dependency_on_a_module_declared_later_still_resolves() {
    // "api" is processed before "core" appears in `modules`, but
    // `build_graph` must still find `core`'s root — module_roots is fully
    // populated in a first pass before any dependency (declared or
    // workspace) is processed.
    let modules = vec![
        module_with_workspace_dependencies("api", vec![], vec!["core".to_string()]),
        module("core", vec![]),
    ];
    let provider = FixturePomProvider::new();

    let graph =
        build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider).expect("should build");

    assert_eq!(graph.module_roots.len(), 2);
}

#[test]
fn workspace_dependency_on_an_unknown_module_is_a_typed_error() {
    let modules = vec![module_with_workspace_dependencies(
        "api",
        vec![],
        vec!["nonexistent".to_string()],
    )];
    let provider = FixturePomProvider::new();

    let result = build_graph(&modules, &HashMap::new(), &HashMap::new(), &provider);

    assert!(matches!(
        result,
        Err(GraphError::UnknownWorkspaceModule { module, dependency })
            if module == "api" && dependency == "nonexistent"
    ));
}
