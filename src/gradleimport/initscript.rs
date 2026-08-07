use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Process-wide counter, not just `std::process::id()` — `import_gradle`
/// can run concurrently more than once inside the same process (e.g.
/// parallel `cargo test` threads), and `process::id()` alone would give
/// two concurrent invocations the same init-script path, racing whichever
/// one deletes its copy first out from under the other's still-running
/// Gradle invocation.
static INVOCATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generates the temporary init-script (docs/architecture.md seção 10, step
/// 1: `jvmfast-model-builder.gradle`) that applies
/// `JvmfastModelBuilderPlugin` to every project in the target build,
/// sourcing the plugin's classes straight from the embedded bridge jar
/// itself (`initscript { dependencies { classpath ... } }`) — no separate
/// plugin artifact to publish or resolve from a repository. Written in
/// Groovy rather than Kotlin DSL: init-scripts are evaluated by Gradle
/// itself, independent of the target build's own DSL, so Groovy works
/// unconditionally regardless of what the target project is written in.
pub fn write_init_script(bridge_jar: &Path) -> io::Result<PathBuf> {
    let invocation = INVOCATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "jvmfast-model-builder-{}-{invocation}.gradle",
        std::process::id()
    ));
    let contents = format!(
        r#"initscript {{
    dependencies {{
        classpath(files("{jar}"))
    }}
}}

allprojects {{
    apply plugin: dev.jvmfast.gradlebridge.JvmfastModelBuilderPlugin
}}
"#,
        jar = escape_groovy_string(&bridge_jar.to_string_lossy())
    );
    std::fs::write(&path, contents)?;
    Ok(path)
}

fn escape_groovy_string(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}
