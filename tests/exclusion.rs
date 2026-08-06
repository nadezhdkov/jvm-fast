use jvmfast::domain::Module;
use jvmfast::exclusion::{is_excluded, merge_exclusions};
use std::collections::HashMap;
use std::path::PathBuf;

fn module_with_exclusions(name: &str, exclusions: HashMap<String, Vec<String>>) -> Module {
    Module {
        name: name.to_string(),
        root: PathBuf::from("."),
        declared_dependencies: Vec::new(),
        boms: Vec::new(),
        exclusions,
    }
}

#[test]
fn is_excluded_true_for_declared_exclusion() {
    let module = module_with_exclusions(
        "core",
        HashMap::from([(
            "org.apache.httpcomponents:httpclient".to_string(),
            vec!["commons-logging:commons-logging".to_string()],
        )]),
    );
    let exclusions = merge_exclusions(&[module]);

    assert!(is_excluded(
        &exclusions,
        "org.apache.httpcomponents:httpclient",
        "commons-logging:commons-logging",
    ));
}

#[test]
fn is_excluded_false_for_undeclared_coordinate() {
    let module = module_with_exclusions(
        "core",
        HashMap::from([(
            "org.apache.httpcomponents:httpclient".to_string(),
            vec!["commons-logging:commons-logging".to_string()],
        )]),
    );
    let exclusions = merge_exclusions(&[module]);

    assert!(!is_excluded(
        &exclusions,
        "org.apache.httpcomponents:httpclient",
        "org.slf4j:slf4j-api",
    ));
}

#[test]
fn is_excluded_false_for_different_parent() {
    let module = module_with_exclusions(
        "core",
        HashMap::from([(
            "org.apache.httpcomponents:httpclient".to_string(),
            vec!["commons-logging:commons-logging".to_string()],
        )]),
    );
    let exclusions = merge_exclusions(&[module]);

    assert!(!is_excluded(
        &exclusions,
        "com.other:library",
        "commons-logging:commons-logging"
    ));
}

#[test]
fn merge_exclusions_combines_single_module() {
    let module = module_with_exclusions(
        "core",
        HashMap::from([(
            "com.example:parent".to_string(),
            vec!["com.example:child".to_string()],
        )]),
    );
    let exclusions = merge_exclusions(&[module]);

    assert!(is_excluded(
        &exclusions,
        "com.example:parent",
        "com.example:child"
    ));
}

#[test]
fn merge_exclusions_unions_across_multiple_modules() {
    let module_a = module_with_exclusions(
        "core",
        HashMap::from([(
            "com.example:parent".to_string(),
            vec!["com.example:child-a".to_string()],
        )]),
    );
    let module_b = module_with_exclusions(
        "api",
        HashMap::from([(
            "com.example:parent".to_string(),
            vec!["com.example:child-b".to_string()],
        )]),
    );
    let exclusions = merge_exclusions(&[module_a, module_b]);

    assert!(is_excluded(
        &exclusions,
        "com.example:parent",
        "com.example:child-a"
    ));
    assert!(is_excluded(
        &exclusions,
        "com.example:parent",
        "com.example:child-b"
    ));
}

#[test]
fn wildcard_star_has_no_special_meaning() {
    let module = module_with_exclusions(
        "core",
        HashMap::from([(
            "org.apache.httpcomponents:httpclient".to_string(),
            vec!["*".to_string()],
        )]),
    );
    let exclusions = merge_exclusions(&[module]);

    // "*" só exclui uma coordenada literalmente igual a "*" — nunca "qualquer coisa".
    assert!(!is_excluded(
        &exclusions,
        "org.apache.httpcomponents:httpclient",
        "commons-logging:commons-logging",
    ));
    assert!(is_excluded(
        &exclusions,
        "org.apache.httpcomponents:httpclient",
        "*"
    ));
}
