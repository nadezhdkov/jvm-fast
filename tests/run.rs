use jvmfast::run::run_main_class;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("jvmfast-test-run-{}-{}", std::process::id(), name))
}

fn compile(dir: &std::path::Path, class_name: &str, source: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let source_path = dir.join(format!("{class_name}.java"));
    std::fs::write(&source_path, source).unwrap();

    let status = std::process::Command::new("javac")
        .arg("-d")
        .arg(dir)
        .arg(&source_path)
        .status()
        .expect("should run javac");
    assert!(status.success(), "javac should compile the fixture class");
}

#[test]
fn run_main_class_executes_and_exits_successfully() {
    let dir = temp_dir("success");
    let _ = std::fs::remove_dir_all(&dir);
    compile(
        &dir,
        "Success",
        "public class Success { public static void main(String[] a) { System.out.println(\"ok\"); } }",
    );

    let status = run_main_class(
        std::path::Path::new("java"),
        std::slice::from_ref(&dir),
        &[],
        "Success",
    )
    .expect("should spawn java");

    assert!(status.success());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_main_class_propagates_non_zero_exit_status() {
    let dir = temp_dir("failure");
    let _ = std::fs::remove_dir_all(&dir);
    compile(
        &dir,
        "Failure",
        "public class Failure { public static void main(String[] a) { System.exit(7); } }",
    );

    let status = run_main_class(
        std::path::Path::new("java"),
        std::slice::from_ref(&dir),
        &[],
        "Failure",
    )
    .expect("should spawn java");

    assert!(!status.success());
    assert_eq!(status.code(), Some(7));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_main_class_passes_jvm_args_before_main_class() {
    let dir = temp_dir("jvm-args");
    let _ = std::fs::remove_dir_all(&dir);
    compile(
        &dir,
        "PrintsProperty",
        "public class PrintsProperty { public static void main(String[] a) { \
         if (!\"bar\".equals(System.getProperty(\"foo\"))) { System.exit(1); } } }",
    );

    let status = run_main_class(
        std::path::Path::new("java"),
        std::slice::from_ref(&dir),
        &["-Dfoo=bar".to_string()],
        "PrintsProperty",
    )
    .expect("should spawn java");

    assert!(status.success());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_main_class_reports_typed_error_for_unknown_binary() {
    let result = run_main_class(
        std::path::Path::new("/does/not/exist/java"),
        &[],
        &[],
        "Whatever",
    );

    assert!(matches!(result, Err(jvmfast::run::RunError::Spawn { .. })));
}
