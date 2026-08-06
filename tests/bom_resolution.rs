use jvmfast::bom::{
    resolve_boms, BomResolutionError, ManagedDependencyEntry, ParsedPom, PomProvider,
};
use jvmfast::domain::BomReference;
use std::collections::HashMap;

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
        entries: Vec<ManagedDependencyEntry>,
    ) -> Self {
        self.poms.insert(
            (coordinate.to_string(), version.to_string()),
            ParsedPom {
                managed_dependencies: entries,
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

fn managed(coordinate: &str, version: &str) -> ManagedDependencyEntry {
    ManagedDependencyEntry {
        coordinate: coordinate.to_string(),
        version: version.to_string(),
        is_bom_import: false,
    }
}

fn imported_bom(coordinate: &str, version: &str) -> ManagedDependencyEntry {
    ManagedDependencyEntry {
        coordinate: coordinate.to_string(),
        version: version.to_string(),
        is_bom_import: true,
    }
}

fn bom_ref(coordinate: &str, version: &str) -> BomReference {
    BomReference {
        coordinate: coordinate.to_string(),
        version: version.to_string(),
    }
}

#[test]
fn single_bom_builds_coordinate_to_version_table() {
    let provider = FixturePomProvider::new().with_pom(
        "com.fasterxml.jackson:jackson-bom",
        "2.17.0",
        vec![
            managed("com.fasterxml.jackson.core:jackson-databind", "2.17.0"),
            managed("com.fasterxml.jackson.core:jackson-core", "2.17.0"),
        ],
    );
    let root_boms = vec![bom_ref("com.fasterxml.jackson:jackson-bom", "2.17.0")];

    let table = resolve_boms(&root_boms, &provider).expect("should resolve");

    assert_eq!(
        table.get("com.fasterxml.jackson.core:jackson-databind"),
        Some(&"2.17.0".to_string())
    );
    assert_eq!(
        table.get("com.fasterxml.jackson.core:jackson-core"),
        Some(&"2.17.0".to_string())
    );
}

#[test]
fn first_bom_listed_wins_on_conflict() {
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:bom-a",
            "1.0",
            vec![managed("com.example:foo", "1.0")],
        )
        .with_pom(
            "com.example:bom-b",
            "1.0",
            vec![managed("com.example:foo", "2.0")],
        );
    let root_boms = vec![
        bom_ref("com.example:bom-a", "1.0"),
        bom_ref("com.example:bom-b", "1.0"),
    ];

    let table = resolve_boms(&root_boms, &provider).expect("should resolve");

    assert_eq!(table.get("com.example:foo"), Some(&"1.0".to_string()));
}

#[test]
fn first_entry_within_same_bom_wins() {
    let provider = FixturePomProvider::new().with_pom(
        "com.example:bom-a",
        "1.0",
        vec![
            managed("com.example:foo", "1.0"),
            managed("com.example:foo", "2.0"),
        ],
    );
    let root_boms = vec![bom_ref("com.example:bom-a", "1.0")];

    let table = resolve_boms(&root_boms, &provider).expect("should resolve");

    assert_eq!(table.get("com.example:foo"), Some(&"1.0".to_string()));
}

#[test]
fn transitive_bom_import_is_flattened_into_table() {
    let provider = FixturePomProvider::new()
        .with_pom(
            "com.example:bom-a",
            "1.0",
            vec![
                imported_bom("com.example:bom-b", "1.0"),
                managed("com.example:own", "1.0"),
            ],
        )
        .with_pom(
            "com.example:bom-b",
            "1.0",
            vec![managed("com.example:imported", "3.0")],
        );
    let root_boms = vec![bom_ref("com.example:bom-a", "1.0")];

    let table = resolve_boms(&root_boms, &provider).expect("should resolve");

    assert_eq!(table.get("com.example:own"), Some(&"1.0".to_string()));
    assert_eq!(table.get("com.example:imported"), Some(&"3.0".to_string()));
}

#[test]
fn import_depth_limit_is_enforced() {
    let mut provider = FixturePomProvider::new();
    for i in 0..=9 {
        let this_coord = format!("com.example:bom-{i}");
        let next_coord = format!("com.example:bom-{}", i + 1);
        provider = provider.with_pom(&this_coord, "1.0", vec![imported_bom(&next_coord, "1.0")]);
    }
    provider = provider.with_pom(
        "com.example:bom-10",
        "1.0",
        vec![imported_bom("com.example:bom-11", "1.0")],
    );

    let root_boms = vec![bom_ref("com.example:bom-0", "1.0")];
    let result = resolve_boms(&root_boms, &provider);

    assert!(matches!(
        result,
        Err(BomResolutionError::ImportDepthExceeded { .. })
    ));
}

#[test]
fn fetch_failure_is_wrapped_as_typed_error() {
    let provider = FixturePomProvider::new();
    let root_boms = vec![bom_ref("com.example:does-not-exist", "1.0")];

    let result = resolve_boms(&root_boms, &provider);

    assert!(matches!(result, Err(BomResolutionError::Fetch { .. })));
}
