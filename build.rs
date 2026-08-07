// Builds `gradle-bridge/` (the one non-Rust component in the stack, seção
// 10) as part of `cargo build` and embeds the resulting jar into the
// `jvmfast` binary via `include_bytes!` — see `src/gradlebridge/mod.rs` for
// how it's extracted to disk at runtime, and CLAUDE.md's Fase 4 writeup for
// why embedding (rather than a runtime download) was picked: it keeps the
// bridge jar's distribution self-contained in the binary, at the cost of
// `cargo build` now requiring a JDK and Gradle's own network bootstrap —
// the same JDK dependency `cargo test` already had for Fase 3's
// javac/java-shelling tests, just widened to `cargo build` too.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bridge_dir = manifest_dir.join("gradle-bridge");

    println!(
        "cargo:rerun-if-changed={}",
        bridge_dir.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bridge_dir.join("build.gradle.kts").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        bridge_dir.join("settings.gradle.kts").display()
    );

    let gradlew_name = if cfg!(windows) {
        "gradlew.bat"
    } else {
        "./gradlew"
    };
    // `shadowJar`, not `jar` — the embedded jar doubles as the Tooling API
    // client-side driver (`Main.java`, invoked as `java -jar` from
    // `src/gradleimport/`), which needs `gradle-tooling-api` and its
    // transitive deps actually present on the classpath at runtime, not
    // just compileOnly like the plugin/model classes. `shadowJar` is
    // classified `-all` (`build.gradle.kts`) precisely so it never
    // collides on disk with the plain `jar` task's output.
    let status = Command::new(gradlew_name)
        .args(["shadowJar", "--no-daemon", "-q"])
        .current_dir(&bridge_dir)
        .status()
        .unwrap_or_else(|source| {
            panic!(
                "failed to invoke `{gradlew_name} shadowJar` in {}: {source}\n\n\
                 jvmfast-gradle-bridge.jar is built as part of `cargo build` and embedded into \
                 the jvmfast binary (see build.rs) — this requires a JDK on PATH and network \
                 access for Gradle's own wrapper bootstrap on first run.",
                bridge_dir.display()
            )
        });
    if !status.success() {
        panic!(
            "`{gradlew_name} shadowJar` failed (exit status: {status}) in {}",
            bridge_dir.display()
        );
    }

    let libs_dir = bridge_dir.join("build/libs");
    let jar_path = std::fs::read_dir(&libs_dir)
        .unwrap_or_else(|source| panic!("failed to read {}: {source}", libs_dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-all.jar"))
        })
        .unwrap_or_else(|| {
            panic!(
                "no `*-all.jar` (shadowJar output) found in {}",
                libs_dir.display()
            )
        });

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR set by cargo"));
    let embedded_path = out_dir.join("jvmfast-gradle-bridge.jar");
    std::fs::copy(&jar_path, &embedded_path).unwrap_or_else(|source| {
        panic!(
            "failed to copy {} to {}: {source}",
            jar_path.display(),
            embedded_path.display()
        )
    });
}
