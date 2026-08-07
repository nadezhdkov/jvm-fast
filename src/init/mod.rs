mod error;

pub use error::InitError;

use std::path::Path;

const MAIN_JAVA_TEMPLATE: &str = "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, World!\");\n    }\n}\n";

/// Notes surfaced to the user about defaults that were applied (seção 9.2
/// doesn't require reporting these, but staying silent about a derived
/// name/java-version would be surprising — same "typed, visible, not
/// silently guessed" spirit as `crate::import`'s `ImportReport`).
pub struct InitReport {
    pub notes: Vec<String>,
}

/// `jvmfast init` (docs/architecture.md seção 9.2): creates a minimal
/// `project.toml` in `project_dir`, plus `src/main/java`/`src/test/java`
/// (and a `Main.java` "Hello, World!" placeholder, unless
/// `src/main/java` already has `.java` files in it — a re-run after
/// manually deleting just `project.toml` shouldn't clobber real source).
///
/// Never overwrites an existing `project.toml`
/// (`InitError::ManifestAlreadyExists`) and refuses to run at all when a
/// `pom.xml` is already present (`InitError::PomXmlDetected`) — seção
/// 9.2 point 5: importing an existing Maven project should go through
/// `jvmfast import-pom`, not a from-scratch manifest that would silently
/// drop its declared dependencies.
///
/// `name`/`java_version` mirror `jvmfast init --name/--java-version`
/// (seção 9.2). Deliberately **not** interactive when omitted (the doc
/// mentions `jvmfast init` alone prompting for both) — this project
/// avoids stdin-blocking as *core* command behavior (only `jdk::confirm_install`
/// uses it, and only as an opt-out confirmation, never as the only way to
/// supply a required value) since a hung prompt in a non-terminal
/// invocation (CI, a test binary) is worse than a sane non-interactive
/// default: `name` defaults to `project_dir`'s directory name,
/// `java_version` defaults to `"lts"` (the same alias `[project].java-version`
/// already resolves everywhere else in the codebase).
pub fn init_project(
    project_dir: &Path,
    name: Option<&str>,
    java_version: Option<&str>,
) -> Result<InitReport, InitError> {
    let manifest_path = project_dir.join("project.toml");
    if manifest_path.exists() {
        return Err(InitError::ManifestAlreadyExists(manifest_path));
    }

    let pom_path = project_dir.join("pom.xml");
    if pom_path.exists() {
        return Err(InitError::PomXmlDetected(pom_path));
    }

    let mut notes = Vec::new();

    let derived_name;
    let name = match name {
        Some(name) => name,
        None => {
            derived_name = derive_name(project_dir)?;
            notes.push(format!(
                "no --name given, derived \"{derived_name}\" from the directory name"
            ));
            &derived_name
        }
    };

    let java_version = match java_version {
        Some(version) => version,
        None => {
            notes.push("no --java-version given, defaulted to \"lts\"".to_string());
            "lts"
        }
    };

    let main_java_dir = project_dir.join("src/main/java");
    let test_java_dir = project_dir.join("src/test/java");
    std::fs::create_dir_all(&main_java_dir).map_err(|source| InitError::Io {
        path: main_java_dir.clone(),
        source,
    })?;
    std::fs::create_dir_all(&test_java_dir).map_err(|source| InitError::Io {
        path: test_java_dir.clone(),
        source,
    })?;

    let has_existing_sources =
        dir_has_java_files(&main_java_dir).map_err(|source| InitError::Io {
            path: main_java_dir.clone(),
            source,
        })?;

    let main_class = if has_existing_sources {
        notes.push(
            "src/main/java already contains .java files — skipped writing a Main.java placeholder"
                .to_string(),
        );
        None
    } else {
        let main_java_path = main_java_dir.join("Main.java");
        std::fs::write(&main_java_path, MAIN_JAVA_TEMPLATE).map_err(|source| InitError::Io {
            path: main_java_path,
            source,
        })?;
        Some("Main")
    };

    let manifest = render_manifest(name, java_version, main_class);
    std::fs::write(&manifest_path, manifest).map_err(|source| InitError::Io {
        path: manifest_path,
        source,
    })?;

    Ok(InitReport { notes })
}

fn derive_name(project_dir: &Path) -> Result<String, InitError> {
    let canonical = project_dir.canonicalize().map_err(|source| InitError::Io {
        path: project_dir.to_path_buf(),
        source,
    })?;
    canonical
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| InitError::CouldNotDeriveName(project_dir.to_path_buf()))
}

fn dir_has_java_files(dir: &Path) -> std::io::Result<bool> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "java") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn render_manifest(name: &str, java_version: &str, main_class: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("[project]\n");
    out.push_str(&format!("name = {}\n", quote(name)));
    out.push_str("version = \"0.1.0\"\n");
    out.push_str(&format!("java-version = {}\n", quote(java_version)));

    if let Some(main_class) = main_class {
        out.push_str("\n[run]\n");
        out.push_str(&format!("main-class = {}\n", quote(main_class)));
    }

    out.push_str("\n[dependencies]\n");

    out
}

fn quote(raw: &str) -> String {
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
