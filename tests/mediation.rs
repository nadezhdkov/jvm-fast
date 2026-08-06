use jvmfast::domain::{MediationReason, NodeId, VersionRequest};
use jvmfast::graph::{CandidateGraph, CandidateNode};
use jvmfast::mediation::mediate;
use std::collections::HashMap;

fn request(version: &str, origin_module: &str, depth: u32) -> VersionRequest {
    VersionRequest {
        version: version.to_string(),
        origin_module: origin_module.to_string(),
        depth,
    }
}

fn single_node_graph(coordinate: &str, requests: Vec<VersionRequest>) -> CandidateGraph {
    let mut candidates = HashMap::new();
    candidates.insert(
        NodeId(0),
        CandidateNode {
            coordinate: coordinate.to_string(),
            requests,
        },
    );
    CandidateGraph {
        candidates,
        edges: Vec::new(),
        module_roots: HashMap::new(),
    }
}

fn resolved(result: &jvmfast::mediation::MediationResult) -> &jvmfast::domain::ResolvedNode {
    result.graph.nodes.get(&NodeId(0)).unwrap()
}

#[test]
fn single_request_is_not_mediated() {
    let graph = single_node_graph("com.example:foo", vec![request("1.0.0", "core", 1)]);

    let result = mediate(graph);

    let node = resolved(&result);
    assert_eq!(node.selected, "1.0.0");
    assert_eq!(node.mediation_reason, MediationReason::SingleRequest);
}

#[test]
fn nearest_depth_wins_over_higher_version() {
    let graph = single_node_graph(
        "com.exemplo:commons",
        vec![request("1.8", "core", 1), request("2.0", "core", 2)],
    );

    let result = mediate(graph);

    let node = resolved(&result);
    assert_eq!(node.selected, "1.8");
    assert_eq!(
        node.mediation_reason,
        MediationReason::NearestDepthWins {
            rejected: vec!["2.0".to_string()]
        }
    );
}

#[test]
fn same_depth_higher_version_wins() {
    let graph = single_node_graph(
        "com.example:foo",
        vec![request("1.8.0", "core", 2), request("2.0.0", "api", 2)],
    );

    let result = mediate(graph);

    let node = resolved(&result);
    assert_eq!(node.selected, "2.0.0");
    assert_eq!(
        node.mediation_reason,
        MediationReason::HigherVersionWins {
            rejected: vec!["1.8.0".to_string()]
        }
    );
}

#[test]
fn version_comparison_is_numeric_not_lexicographic() {
    // "2.9.0" > "2.10.0" as plain strings, but 2.10.0 is the real higher version.
    let graph = single_node_graph(
        "com.example:foo",
        vec![request("2.9.0", "core", 1), request("2.10.0", "core", 1)],
    );

    let result = mediate(graph);

    assert_eq!(resolved(&result).selected, "2.10.0");
}

#[test]
fn same_version_from_different_modules_uses_deterministic_tie_break() {
    let graph = single_node_graph(
        "com.example:shared",
        vec![request("1.0.0", "core", 1), request("1.0.0", "api", 1)],
    );

    let result = mediate(graph);

    let node = resolved(&result);
    assert_eq!(node.selected, "1.0.0");
    assert_eq!(
        node.mediation_reason,
        MediationReason::DeterministicTieBreak {
            rejected: vec!["1.0.0".to_string()]
        }
    );
}

#[test]
fn unparseable_versions_fall_back_to_deterministic_string_comparison() {
    let graph = single_node_graph(
        "com.example:foo",
        vec![
            request("RELEASE", "core", 1),
            request("SNAPSHOT", "core", 1),
        ],
    );

    let result = mediate(graph);

    // "SNAPSHOT" > "RELEASE" lexicographically — not "correct" Maven semantics,
    // just a documented, deterministic fallback for non-semver strings.
    assert_eq!(resolved(&result).selected, "SNAPSHOT");
}

#[test]
fn edges_pass_through_mediation_unchanged() {
    let mut candidates = HashMap::new();
    candidates.insert(
        NodeId(1),
        CandidateNode {
            coordinate: "com.example:foo".to_string(),
            requests: vec![request("1.0.0", "core", 1)],
        },
    );
    let graph = CandidateGraph {
        candidates,
        edges: vec![jvmfast::domain::GraphEdge {
            from: NodeId(0),
            to: NodeId(1),
            kind: jvmfast::domain::EdgeKind::ModuleDeclared,
        }],
        module_roots: HashMap::from([("core".to_string(), NodeId(0))]),
    };

    let result = mediate(graph);

    assert_eq!(result.graph.edges.len(), 1);
    assert_eq!(result.module_roots.get("core"), Some(&NodeId(0)));
}
