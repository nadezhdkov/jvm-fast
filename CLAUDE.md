# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

**Fase 1 (docs/architecture.md seção 12 — "resolução e cache") is
complete.** The `jvmfast` binary resolves `project.toml`, downloads
artifacts over real HTTP, and writes/reads `project.lock`, end to end, via
`install`/`update`/`add`/`remove`/`tree`/`why`. **Fase 2 (JDK management,
seção 7) is complete**: `jvmfast jdk install <major>`/`jdk list`/`jdk use`
all work end to end against the real Eclipse Temurin/Adoptium API and a
real (if narrowly-scoped) `~/.config/jvmfast/config.toml`; `[project].java-
version` (including the `"lts"` alias) is now resolved and auto-installed
(with interactive confirmation unless `--yes`) as part of `jvmfast
install`/`update`, and the concrete resolved version is persisted in
`project.lock` (`Lockfile.java_version`) so the alias doesn't silently
re-resolve on every build. **Fase 3 (build/run/test, seção 8/8.1) is
complete**: `jvmfast build` compiles `src/main/java` with `javac` from the
project's resolved JDK and copies `src/main/resources` into
`target/classes`, requiring a valid `project.lock` and never
resolving/downloading/installing anything itself (typed errors point at
`install`/`jdk install` instead); `jvmfast run` builds on top of it
(always recompiles, executes `[run].main-class`/`jvm-args`, stdio
inherited); `jvmfast test` compiles `src/test/java` against
`target/classes` + `[dev-dependencies]` (resolved fresh every run, not
lockfile-pinned yet) and runs it via the JUnit Platform Console Standalone
— treated as jvm-fast's own internal dependency, downloaded straight from
Maven Central and cached like any artifact, never declared in
`project.toml`. All three verified end to end against a real system JDK
and (for `test`'s console-launcher download) real Maven Central — that
testing surfaced two significant pre-existing Fase 1 limitations, both
now addressed: `download::fetch_checksum`'s `.sha256`-only assumption is
**fixed** (falls back to `.sha1`, which is what most real Maven Central
artifacts actually publish — see "Known, deliberate gaps inside Fase 1"
below for the full writeup), and `graph::build_graph` not filtering POM
dependencies by `<scope>` is **also fixed** (test/provided/system-scoped
transitives no longer leak into the graph as if compile-scoped — same
section below). **Fase 4 (interop, seção 10) is complete**: `jvmfast
import-pom [pom.xml] [-o project.toml]` reads an existing `pom.xml` and
writes an equivalent `project.toml`, never touching the source `pom.xml`
and never overwriting an existing manifest
(`ImportError::ManifestAlreadyExists`) — the two files can coexist during
a transition, per seção 10. It preserves dependencies (including
`${property}` interpolation, resolved in-place, no parent-POM
inheritance), `test`-scoped dependencies as `[dev-dependencies]`,
`<dependencyManagement>` BOM imports as `[boms]`, per-dependency
`<exclusions>` as `[exclusions]`, and `<repositories>` (first declared
becomes `default`, matching how resolution already only reads that key —
see the existing multi-repository-fallback gap below). `java-version` is
resolved from `maven.compiler.release`/`.target`/`.source`/`java.version`
properties, falling back to `"lts"` with a report note when none are
declared. Maven version ranges (`[1.0,2.0)`, `[1.5,)`, `[1.0]`...) are
translated only when there's a direct equivalent — today that's just a
single pinned value (`[1.0]` → `1.0`); every open-ended or multi-segment
range is reported as needing manual attention rather than guessed, since
computing "the greatest version satisfiable at import time" would need a
`maven-metadata.xml` lookup that doesn't exist yet (same gap as `jvmfast
add` without an explicit version). Anything else without a jvm-fast
equivalent — `provided`/`system`-scoped dependencies, unresolved
properties, `<profiles>`, `<build><plugins>`, repositories beyond the
first — is skipped from the generated manifest and surfaced as a report
note instead of silently dropped or guessed. `jvmfast import-gradle`
(Tooling API bridge, seção 10) is implemented end to end too, against a
real Gradle build via a real Tooling API connection: [`gradle-bridge/`](gradle-bridge/)
(a standalone Gradle project — own build, own `gradlew`, own CI job) now
ships `JvmfastModelBuilder.buildAll` walking `project.getConfigurations()`
(`compileClasspath`/`runtimeClasspath`/`testCompileClasspath`) into a real
`JvmfastDependencyModel`, plus a `Main` class (the Tooling API
*client*-side driver, `dev.jvmfast.gradlebridge.Main`) that opens a real
`GradleConnector` connection to the target project and prints the
resolved model as JSON on its own stdout — never `gradlew`'s, per seção
10's explicit rejection of stdout-text-parsing. `build.rs` builds that jar
(as a `shadowJar`, since the client-side driver needs its own bundled copy
of `gradle-tooling-api`) and embeds it into the `jvmfast` binary
(`src/gradlebridge::extract_bridge_jar` gets it onto disk at runtime,
content-addressed like any other cached artifact); `src/gradleimport/`
generates the init-script (seção 10 step 1), invokes the extracted jar as
a `java -jar` subprocess, parses its JSON stdout, and writes an equivalent
`project.toml` — `[dependencies]` from `compileClasspath`/`runtimeClasspath`,
`[dev-dependencies]` from whatever `testCompileClasspath` adds beyond
that, `java-version` defaulted to `"lts"` (not exposed by the model yet),
and always a report note about seção 10's documented mediation-divergence
risk (Gradle resolves highest-version-wins; `jvmfast update` afterward
uses jvm-fast's own nearest-depth-wins and may pick different versions).
Verified against a real Gradle 9.6.1 build resolving real dependencies
from real Maven Central (`tests/fixtures/gradle/simple-project/`, its own
committed `gradlew`) — the same deliberate, narrow network/real-tool
exception this repo already makes for `build`/`run`/`test`'s real
JDK and `test`'s real-Maven-Central console-jar download. See the Fase 4
writeup below for the full breakdown. See "Roadmap" below for the
specific gaps left inside Fase 1 (targeted `update <coord>`, `add`
without an explicit version, editing `[dev-dependencies]` from the CLI,
multi-repository fallback, per-host download throttling), Fase 2
(exact-version JDK install, listing *available* (not just installed)
JDKs, and global `config.toml` beyond `[defaults]`), Fase 3 (see below),
and Fase 4 (multi-project Gradle builds, `java-version` extraction from
Gradle, and `import-pom`'s parent-POM-inheritance gap) — each is a typed,
rejected-not-faked error or a documented report note today, not silent
scope creep. **Fase 5 (workspace e multi-módulo, seção 12) is complete**:
`[workspace].members` in the root `project.toml` is real —
`workspace::load_workspace` loads every declared member as a genuine
`Module` from its own `<member>/project.toml` — and a module can now
declare `[workspace-dependencies]` on another module in the same
workspace, which `graph::build_graph` turns into a real
`EdgeKind::WorkspaceModule` edge and `build::build` turns into both
correct topological compile ordering (`build::module_order`) and an
inter-module classpath (a dependency's `target/classes` reaches its
dependent's `javac -cp`) — verified end to end compiling one module's
source against a class only another module defines, with real `javac`.
`jvmfast test` was reconciled to scope its own compile classpath the same
explicit, `workspace_dependencies`-based way instead of its old cruder
implicit accumulation. Build is incremental at module granularity too
(`build::fingerprint`, content-hash-based, correctly propagating
invalidation transitively through workspace dependencies) — `jvmfast
build`/`run`/`test` all skip recompiling a module whose inputs haven't
changed. `jvmfast run`/`jvmfast test` both gained `--module <name>` (any
module, root or member, can declare its own `[run]`/`[dev-dependencies]`;
omitting the flag defaults to the root module, so single-module workspaces
are entirely unaffected) — see the Fase 5 writeup below for the full
breakdown.

- [`docs/architecture.md`](docs/architecture.md) — the full architecture spec
  for jvm-fast, a native Rust CLI ("uv for Java") for dependency management,
  JDK management, and build/run/test of single-module Java projects. This is
  the source of truth for design decisions; it's organized in numbered
  sections (referenced as "seção N" throughout the repo) and is written in
  Portuguese.
- [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) — practical coding/commit
  conventions, plus a README template for internal Rust crates (only
  applicable if/when the project adopts a multi-crate workspace — not yet
  decided; today it's a single binary crate with a `lib.rs`/`main.rs` split).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — human-contributor-facing setup/
  testing/lint/process guide (English); overlaps with `docs/CONVENTIONS.md`
  on purpose (same rules, different audience) — if the two ever disagree,
  `docs/CONVENTIONS.md` and the actual CI config (`.github/workflows/`)
  win, since `CONTRIBUTING.md` is describing them, not defining them.
- [`STYLE.md`](STYLE.md) — prose/CLI-messaging style guide; **partially
  aspirational** (colored output, `--verbose`/logging levels, hints — none
  implemented in `src/cli/` yet), same "spec ahead of code" spirit as
  `docs/architecture.md`. Don't infer that a behavior it describes exists
  without checking the code.
- `AGENTS.md` — coding-agent-specific conventions (test style, error
  typing, the real-JDK/real-Maven-Central test exceptions); consistent
  with this file, kept separate because tools other than Claude Code read
  `AGENTS.md` by convention.

## Build, test, lint

- `cargo build` — build the `jvmfast` binary. **Requires a JDK on `PATH`
  and network access** since Fase 4: `build.rs` builds
  [`gradle-bridge/`](gradle-bridge/) (`./gradlew shadowJar`) and embeds
  the resulting jar into the binary via `include_bytes!` — see the Fase 4
  writeup below.
- `cargo test` — run all tests (integration tests for manifest parsing live
  in `tests/manifest_parsing.rs`, using fixtures under `tests/fixtures/`).
  **Requires a real `javac`/`java` on `PATH`** since `tests/build.rs`/
  `tests/cli_build.rs` (Fase 3) shell out to the system JDK, not a mock;
  CI installs one via `actions/setup-java` (see `.github/workflows/rust.yml`)
- `cargo test <name>` — run a single test by name substring (e.g. `cargo
  test bom_managed_dependency`)
- `cargo clippy --all-targets -- -D warnings` — lint; CI fails on any
  warning (see `docs/CONVENTIONS.md` — no silently-suppressed `#[allow(...)]`)
- `cargo fmt --all` — format; `cargo fmt --all -- --check` to verify without
  writing (what CI runs)

## Roadmap — what's implemented vs. next

**Fase 1 is fully implemented**, module by module:

- `src/domain/` — the seção 3.1 domain types (`Module`, `Dependency`,
  `VersionReq`, `BomReference`, `DependencyGraph`, `GraphEdge`,
  `ResolvedNode`, `Lockfile`, `Workspace`/`WorkspaceConfig`), all with real
  constructors now (`workspace::load_workspace`, `mediation::mediate`).
- `src/manifest/` — `parse_module`/`parse_repositories` parse `project.toml`
  into `Module` + a raw `[repositories]` map (kept separate from `Module`
  since seção 3.1 doesn't model repositories in the domain).
- `src/version/` — `SemVer`, `VersionRequirement::parse` for exact/`^`/`~`
  (seção 6.1). Not yet wired against real "available versions" metadata —
  see `GraphError::UnresolvedVersionRange` below.
- `src/bom/`, `src/exclusion/`, `src/pom/` — BOM table resolution (seção
  3.3, first-BOM-wins/first-entry-wins, depth-10 import limit),
  parent/candidate exclusion checks (seção 3.4, no wildcard support), and
  real `quick-xml` POM parsing, all behind the shared `PomProvider` trait.
- `src/graph/` + `src/mediation/` — `build_graph` walks transitives via
  `PomProvider` into a `CandidateGraph`; `mediate` turns that into the real
  `DependencyGraph`/`ResolvedNode` with fixed precedence `depth ASC →
  version DESC → deterministic tie-break` (seção 6.2/13.1). Each
  `VersionRequest` is its own mediation candidate, never deduplicated by
  version string first.
- `src/resolve/` — `resolve(modules, provider)` is the first function that
  chains BOM resolution → exclusions → graph → mediation end to end; used
  by every CLI command that needs a resolved graph.
- `src/lockfile/` + `src/workspace/` — `compute_manifest_hash`,
  `is_lockfile_valid`, `build_lockfile`, `read_lockfile`/`write_lockfile`,
  and `load_workspace`/`current_manifest_hash` (the latter recomputes the
  hash separately from whatever's loaded from an existing lock, since
  `Workspace` doesn't carry both).
- `src/cache/` — content-addressable `CacheStore` (SHA-256 two-level
  sharding, atomic temp-file→verify→rename writes) + a `rusqlite`-backed
  `index.db`.
- `src/maven/` — shared Maven repository layout (`artifact_path`/
  `artifact_url`/`artifact_filename`), used by both `pom::HttpPomProvider`
  and `download`.
- `src/download/` — `DownloadClient` (the codebase's first `async` code,
  `tokio` + `reqwest`): `download_many` for concurrency-capped parallel JAR
  downloads (seção 6.2 passo 6), `fetch_checksum` for the `.sha256` sidecar
  a repository publishes next to each artifact (used when a lock doesn't
  exist yet, so there's no `sha256` to verify against beforehand). Both
  `download` and `pom::HttpPomProvider` are tested against a hand-rolled
  local mock HTTP server (`tests/support/mod.rs`), never real Maven
  Central, per CONVENTIONS.md.
- `src/cli/` — the orchestrator and `clap` subcommands:
  `install`/`update`/`add`/`remove`/`tree`/`why`. `install::install` is the
  first place `workspace::load_workspace` → `lockfile::is_lockfile_valid` →
  `resolve::resolve` (with `pom::HttpPomProvider`) →
  `download::DownloadClient::download_many` → `lockfile::build_lockfile` →
  `lockfile::write_lockfile` actually runs end to end; when the lock is
  already valid, it skips resolution entirely and downloads straight from
  `LockedPackage.resolved_from`/`sha256`, per the seção 6 flowchart.
  `add`/`remove` edit `project.toml` via `toml_edit` (preserves comments/
  formatting) then re-resolve. `tree`/`why` are pure-formatting functions
  (`format_tree`/`format_why`) over an in-memory `DependencyGraph`, fully
  unit-tested without I/O.

**Fase 2 (JDK management, seção 7) — complete**:

- `src/jdk/` — `AdoptiumClient::latest_release` queries the real Eclipse
  Temurin/Adoptium public API (`https://api.adoptium.net`, seção 7:
  "Distribuição padrão: Eclipse Temurin") for the latest release of a
  major/feature version; `jdk::install` downloads the `.tar.gz`, verifies
  its SHA-256 against the checksum the API itself reports, and extracts it
  atomically (temp dir → verify → rename, the same discipline as
  `cache::CacheStore::write_artifact`, seção 5.1, adapted from a single
  file to a directory tree) into `~/.cache/jvmfast/jdks/<version>-tem/`
  (seção 5's documented cache tree). `jdk::list_installed` scans that
  directory. Tested against a hand-rolled local mock HTTP server serving a
  crafted JSON response + a real (small, in-test-built) `.tar.gz` — the
  exact Adoptium v3 JSON shape is assumed from public API docs, not
  verified against production (no outbound network access in this
  environment), same caveat as `pom::HttpPomProvider`'s Maven-layout
  assumption in Fase 1.
- `src/config/` — `load_defaults`/`write_default_java_version` read/write
  just the `[defaults]` table of `~/.config/jvmfast/config.toml` (seção
  3.5), via `domain::DefaultsConfig` (now `Serialize`/`Deserialize`, kebab-
  case `java-version`) — `write_default_java_version` edits in place via
  `toml_edit`, same non-destructive discipline as `cli::edit` for
  `project.toml`. **Narrower than seção 3.5 as a whole**: `[network]`/
  `[output]` aren't read by anyone yet, and `workspace::load_workspace`
  still only uses `WorkspaceConfig::default()` — overlaying the full
  documented precedence chain (CLI flags → env → project.toml →
  config.toml → hardcoded defaults) is a separate, bigger milestone.
- `manifest::parse_java_version` — reads `[project].java-version` straight
  from `project.toml`, same pattern as `parse_repositories` (`Module`,
  seção 3.1, has no field for it either; this is resolution *configuration*,
  not dependency *declaration*).
- `src/cli/jdk.rs` — wires `jvmfast jdk install <major>`, `jdk list`
  (marks which installed version matches the configured
  `[defaults].java-version`, if any), and `jdk use <major>` (writes
  `[defaults].java-version` — rejects a major version that isn't installed
  yet via `CliError::JavaVersionNotInstalled`, since pointing the default
  at nothing would be a silent inconsistent state). Also
  `resolve_project_java_version`/`ensure_project_jdk` (seção 7): resolve
  `[project].java-version` (via `manifest::parse_java_version` +
  `jdk::resolve_feature_version`, which hits Adoptium's
  `/v3/info/available_releases` only for the `"lts"` alias) and make sure
  that JDK is installed, prompting for confirmation on stdin unless
  `yes=true` (`--yes`, wired from `Command::Install`/`Command::Update`) —
  declining returns `CliError::JdkInstallDeclined`. `cli::install::install`
  calls `resolve_project_java_version` when actually (re)generating the
  lock (so `"lts"` gets resolved fresh) and `ensure_project_jdk` with the
  already-persisted `Lockfile.java_version` when reusing a valid lock (so
  the alias is *not* re-resolved on every build, per seção 3's
  documented rule — only `jvmfast update` reassesses it). `add`/`remove`
  always behave as `--yes` for this step, since blocking a dependency edit
  on an interactive JDK prompt would be surprising.
- `src/domain/lockfile.rs` — `Lockfile` gained a `java-version` field (the
  concrete major version selected at resolution time, never the `"lts"`
  alias itself); `lockfile::build_lockfile` takes it as a parameter.

**Fase 3 (build/run/test, seção 8/8.1) — complete**:

- `src/build/` — `build::build(workspace, javac, cache_root)` (seção 8):
  iterates `workspace.modules` (never indexes `[0]`, per the Fase 5
  compatibility rules below) and, per module, compiles
  `src/main/java` with `javac -d target/classes -cp <classpath>
  <sources...>` then copies `src/main/resources` into `target/classes`,
  preserving relative structure — matching seção 8's "no separate merge
  step" rule. `build::classpath::locked_classpath` builds the classpath
  entirely from `project.lock` (`CacheStore::artifact_path` per
  `LockedPackage`, seção 5) — never re-resolves or touches the network; a
  package listed in the lock but missing from the cache is
  `BuildError::MissingArtifact`, pointing at `jvmfast install`, not a
  silently-incomplete classpath. `build::compile` shells out to the real
  `javac` binary via `std::process::Command` (the project's first
  subprocess-spawning code) and reports non-zero exit as
  `BuildError::CompileFailed { stderr }`; a module with zero `.java` files
  is a valid no-op build (still creates `target/classes` for resources to
  land in).
- `src/cli/build.rs` — wires `jvmfast build`: requires `project.lock` to
  exist and be valid (`CliError::LockfileMissing`/`LockfileStale` — build
  never resolves/downloads implicitly), then resolves the project's
  installed JDK via `jdk::find_installed(jdks_root, &lockfile.java_version)`
  (`CliError::JavaVersionNotInstalled` if that JDK was never `jdk
  install`ed) and points `javac` at `<jdks_root>/<installed>/bin/javac`.
- `src/jdk/list.rs` gained `find_installed` (major-version → installed
  directory name lookup), extracted from logic that was duplicated across
  `jdk list`/`jdk use`/`ensure_installed` (Fase 2) and now reused a fourth
  time by `cli::build`.
- Tested against the real system `javac`/`java` (not a mock or a
  downloaded Temurin) — `build` only needs a working `javac` binary at a
  path, so `tests/build.rs`/`tests/cli_build.rs` point it at whatever JDK
  is actually installed in the environment (a real, if incidental,
  integration test — CI needs a JDK on `PATH` for these to run, unlike
  every other test suite in this repo, which never depends on host
  tooling).
- `src/run/` — `run::run_main_class(java, classpath, jvm_args, main_class)`
  (seção 8): the project's second subprocess-spawning module (after
  `build::compile`), invokes `java -cp <classpath> <jvm-args> <main-class>`
  with stdio inherited from the parent process — the user's program output
  goes straight to the terminal, `jvmfast run` never captures/reformats
  it. Deliberately just a thin wrapper around `std::process::Command`; all
  classpath/JDK-selection logic lives in the caller (`cli::run`), same
  split as `build::compile` vs. `cli::build`.
- `src/cli/run.rs` — wires `jvmfast run`: same `project.lock`
  presence/validity checks as `cli::build` (reuses
  `CliError::LockfileMissing`/`LockfileStale`, now phrased generically for
  both commands), reads `[run].main-class`/`jvm-args` via
  `manifest::parse_run_config` (`CliError::MainClassNotConfigured` if
  `main-class` is absent — a manifest without one has nothing for `run` to
  execute), then *always* recompiles via `crate::build::build` (no
  incremental build yet, so "compile if needed" is currently "compile,
  full stop") before executing. Classpath = locked dependencies
  (`build::locked_classpath`, now `pub` from `src/build/`) + every
  module's `target/classes`. A non-zero exit from the user's program
  becomes `CliError::ProgramExited(code)` (`-1` sentinel if the process
  was killed by a signal, so no fake exit code is invented) — this repo's
  first command whose success/failure depends on an *external* process's
  outcome, not just its own logic.
- Tested the same way as `build`: `tests/run.rs`/`tests/cli_run.rs` spawn
  the real system `java`, not a mock.
- `src/testing/` (seção 8.1) — `testing::run_tests(...)` orchestrates
  `jvmfast test`: `devdeps::resolve_dev_classpath` resolves and downloads
  `[dev-dependencies]` through the exact same `resolve()`/`DownloadClient`
  pipeline `cli::install` uses for production deps, but against a
  synthetic `Module` (`manifest::parse_dev_module` /
  `convert::to_dev_module`, named `"<project>-test"`, reusing the
  project's `[boms]`/`[exclusions]`) and *never* cached in `project.lock`
  — a deliberate gap, see below. `console::ensure_console_jar` treats
  JUnit Platform Console Standalone (`org.junit.platform:junit-platform-
  console-standalone`, version pinned in `console::CONSOLE_VERSION`) as
  jvm-fast's own internal dependency exactly as seção 8.1 specifies:
  fetched from Maven Central directly (`cli::context::MAVEN_CENTRAL`,
  deliberately *not* the project's own `[repositories].default` — it's a
  jvm-fast tool, not a project dependency) and cached like any other
  artifact, never appearing in `project.toml`. `console::run` invokes
  `java -jar <console.jar> execute --classpath <all> --scan-classpath
  <target/test-classes> ...` with stdio inherited (same discipline as
  `run::run_main_class`) — the exact CLI flags were verified by hand
  against the real 1.14.4 jar (`java -jar ... execute --help` plus actual
  passing/failing/tagged test runs), not assumed from documentation the
  way the Adoptium/Maven-layout integrations had to be.
- `src/testing/filter.rs` translates seção 8.1's jvm-fast-specific
  `--filter` vocabulary (`"tag:fast"` → `--include-tag fast`; anything
  else, e.g. `"*.UserTest"`, treated as a class-name glob →
  `--include-classname` with `*` translated to `.*` and every other regex
  metacharacter escaped, anchored with `^`/`$`) into the Console
  Launcher's real flags — `glob_to_regex`/`parse_filter` are `pub` and
  unit-tested directly (`tests/testing.rs`) since this project never uses
  inline `#[cfg(test)]` modules.
- `src/cli/test.rs` — wires `jvmfast test --filter <spec> [--report-xml]`:
  same `project.lock`/JDK checks as `build`/`run`, recompiles
  `src/main/java` first (test code compiles against `target/classes`),
  then delegates to `testing::run_tests`. `--fail-fast` is rejected
  (`CliError::FailFastNotSupported`) rather than silently ignored — the
  Console Launcher has no native stop-on-first-failure flag to map it to
  (confirmed against the real `--help` output, not assumed). A non-zero
  Console Launcher exit becomes `CliError::TestsFailed(code)`, distinct
  from `build`'s `CompileFailed`/`run`'s `ProgramExited` (partial progress
  on seção 11's "distinct exit code per failure kind", still not wired to
  the process's actual OS exit code — see existing gap).
- `src/build/` gained three more `pub` re-exports (`compile`,
  `copy_resources`, `collect_java_sources`) so `src/testing/` can reuse
  them for `src/test/java`/`src/test/resources` instead of duplicating
  compile/copy logic — same functions `jvmfast build` already used for
  `src/main/*`, just pointed at different directories.
- `src/manifest/convert.rs` — `to_module`/`to_dev_module` now share
  `convert_dependencies`/`convert_boms`/`convert_exclusions` helpers
  (extracted, not duplicated, once a second caller needed the same
  coordinate-validation loops).
- Tested against real Maven Central for the console-jar download
  specifically (`tests/cli_test.rs` — alongside `build`/`run`'s real-JDK
  dependency, this is the only other deliberate, narrow exception in this
  repo to "tests never touch real network"; the *project's own*
  `[repositories].default` in these tests still points at real Maven
  Central too, but only because the fixtures declare zero dependencies,
  so `install` never actually fetches anything from it — dev-dependency
  resolution itself is exercised by the same mock-server-backed pattern
  `cli::install` already uses, not duplicated here).

**Fase 4 (interop, seção 10) — complete: `import-pom` and `import-gradle`
both implemented end to end**:

- `src/pom/xml.rs` — extended (not rewritten) to also capture
  `<project><artifactId>`/`<version>` (direct only, no `<parent>`
  inheritance), `<properties>`, `<repositories>` (`id`/`url` pairs, in
  declaration order), per-dependency `<exclusions>`, and presence flags for
  top-level `<profiles>`/`<build><plugins>` — all additive to `ParsedPom`/
  `PomDependency` (`project_artifact_id`, `project_version`, `properties`,
  `repositories`, `has_profiles`, `has_plugins`, `PomDependency.exclusions`),
  consumed only by `crate::import`; the normal resolution path
  (`crate::graph`/`crate::bom`) never reads any of these new fields, so
  `jvmfast install`/`add`/`test` behavior is unchanged. Text-target
  resolution was refactored from a single depth-tracked `current_field`
  into a `TextTarget` enum recomputed fresh per open tag, so the new
  sibling contexts (exclusion fields, repository fields, project-level
  fields, properties) can't leak state into each other.
- `src/import/` (new module) — `import_pom(pom_path, manifest_path)`:
  reads `pom_path`, refuses to run if `manifest_path` already exists
  (`ImportError::ManifestAlreadyExists`, never overwrites), parses via
  `pom::parse_pom_xml`, and writes a new `project.toml`. Requires a direct
  `<project><artifactId>`/`<version>` (`ImportError::MissingArtifactId`/
  `MissingVersion` otherwise — POMs that rely on `<parent>` inheritance for
  either need to declare them explicitly first, same parent-POM-inheritance
  gap already documented for `pom::xml` since Fase 1).
  - `src/import/range.rs` — `translate_maven_range` (Maven range syntax,
    `[1.0,2.0)`/`[1.5,)`/`(,2.0]`/`[1.0]`, seção 10). Only `[x]` (a single
    pinned value) has a direct jvm-fast equivalence; every open-ended or
    multi-segment range returns `Unresolved` rather than guessing — seção
    10's "maior valor satisfazível no momento do import" would need a
    `maven-metadata.xml` lookup, the same missing infrastructure as
    `jvmfast add` without an explicit version
    (`CliError::VersionOmittedNotSupported`) and
    `GraphError::UnresolvedVersionRange` — not implemented here either, on
    purpose, rather than half-built just for import.
  - `src/import/generate.rs` — `render_manifest`, a pure function (no I/O)
    that formats the `project.toml` text from already-resolved data, kept
    separate from the parsing/interpolation logic in `mod.rs` so the output
    format is testable independent of a real `pom.xml`.
  - `mod.rs`'s `interpolate` resolves `${property}` references (one or more
    per string) against `ParsedPom.properties` — `None` (not a fabricated
    literal `${...}` string) if any referenced key is missing, which the
    caller turns into a report note and skips that dependency/BOM entry
    rather than writing a broken version into the generated manifest.
    Deliberately does not follow `<parent>` POM property inheritance (same
    gap as above).
  - Import mapping decisions: `test`-scoped dependencies become
    `[dev-dependencies]`; `provided`/`system`-scoped dependencies have no
    jvm-fast equivalent and are skipped with a report note;
    `<dependencyManagement>` entries with `<scope>import</scope>` become
    `[boms]` (version interpolated same as any other); a dependency with no
    `<version>` at all is imported as `dependency = true` (BOM-managed) only
    if at least one BOM import was found in `<dependencyManagement>` —
    otherwise it's skipped with a note, since jvm-fast has no local
    (non-BOM) managed-version concept to fall back to; `<repositories>` are
    imported in declaration order with the *first* becoming `default` (the
    only key `crate::graph`'s resolution path actually reads today — see
    the existing multi-repository-fallback gap below) and the rest keyed by
    their `<id>` (or `repo-N` if absent), with a report note counting how
    many extra ones were imported but not yet resolved against.
  - `<profiles>`/`<build><plugins>` presence becomes a report note each
    (seção 10: "reportando quais elementos não têm equivalente... precisam
    de atenção manual") — detected, not parsed in any depth, since only
    their presence (not content) has no jvm-fast equivalent to preserve.
- `src/cli/import.rs` — wires `jvmfast import-pom [pom] [-o path]`
  (`pom` defaults to `pom.xml` at the project root; output is always
  `project.toml` at the root). Prints a one-line-per-note summary of
  everything `import_pom`'s report flagged, if anything did.
- `src/cli/error.rs` — `CliError::Import(#[from] ImportError)`.
- Tested with fixture-only POMs (`tests/fixtures/import/`,
  `tests/fixtures/poms/import_metadata.xml`) — no real network, no real
  Maven Central, consistent with every other parsing-layer test in this
  repo; a `full_pom.xml` fixture exercises every conversion path (plain/
  interpolated/BOM-managed/pinned-range dependency versions,
  provided-scope skip, unresolved-property skip, unresolved-range skip,
  exclusions, dev-dependencies, BOM import, multiple repositories,
  profiles/plugins detection) in one integration test
  (`tests/import.rs`), plus dedicated `tests/import_range.rs` for the
  range-translation boundary and `tests/cli_import.rs` for the CLI wiring
  (default path, explicit path, already-exists rejection, report
  formatting).
- [`gradle-bridge/`](gradle-bridge/) (non-Rust, standalone Gradle
  project — own `build.gradle.kts`, own `gradlew`, own CI job at
  `.github/workflows/gradle-bridge.yml` triggered only on changes under
  this directory) — the full JVM-side implementation of `jvmfast
  import-gradle` (Tooling API bridge, seção 10), server side *and* client
  side in the same jar. Contains:
  - `dev.jvmfast.gradlebridge.model.{JvmfastDependencyModel,JvmfastModule,JvmfastDependency}`
    — the typed model shape both sides of the Tooling API exchange agree
    on, now with real `Default*` `Serializable` implementation classes
    (`DefaultJvmfastDependencyModel`/`DefaultJvmfastModule`/`DefaultJvmfastDependency`)
    alongside the interfaces. `JvmfastModule` gained `getVersion()`
    (`project.getVersion()` as a string, including Gradle's own
    `"unspecified"` default when unset) — needed to fill
    `project.toml`'s `[project].version`, which the original interfaces
    (JVM-side skeleton milestone) didn't carry.
  - `JvmfastModelBuilderPlugin` — unchanged from the skeleton: a
    `Plugin<Project>` taking `ToolingModelBuilderRegistry` via
    constructor injection and registering `JvmfastModelBuilder` against
    it, applied by the init-script `src/gradleimport/` generates (seção
    10 step 1).
  - `JvmfastModelBuilder.buildAll` — **now real**, not a stub: for each of
    `compileClasspath`/`runtimeClasspath`/`testCompileClasspath` (skipped
    silently if the configuration doesn't exist, e.g. no `java` plugin
    applied — never an error), reads `configuration.getIncoming()
    .getResolutionResult().getAllComponents()` — the whole resolved
    dependency graph, direct + transitive, already flattened and
    deduplicated by Gradle itself — and reports each component's
    `group:artifact`, version, and originating configuration as a
    `JvmfastDependency`. Deliberately uses `ResolutionResult` (metadata
    only) rather than `ResolvedConfiguration.getResolvedArtifacts()`,
    which would force actual jar-file resolution for no benefit here.
    Project dependencies (other subprojects, no `ModuleVersionIdentifier`)
    are skipped — multi-project graphs stay Fase 5 scope.
  - `Main` (new) — the Tooling API **client**-side driver (seção 10 steps
    2-4), invoked by `src/gradleimport/` as `java -jar
    jvmfast-gradle-bridge.jar <project-dir> <init-script-path>`. Opens a
    real `GradleConnector.newConnector().forProjectDirectory(...).connect()`,
    requests `JvmfastDependencyModel` with `--init-script
    <init-script-path>`, redirects the target build's own console output
    to *this* process's stderr (`setStandardOutput`/`setStandardError`,
    discardable, never mixed into the result channel), and prints a
    hand-rolled JSON serialization of the model to its own stdout —
    `Main.toJson`, no Gson/Jackson dependency, since the model is a small
    fixed shape of strings/lists and adding a JSON library would be one
    more thing to shade into the client jar for no real benefit. `Main`
    exits non-zero with a message on stderr for `BuildException`
    (`gradlew`-side build failure) and `GradleConnectionException`
    (couldn't connect at all) separately, so `src/gradleimport/` can
    distinguish them.
  - No Gradle toolchain auto-provisioning (`java.toolchain {}`) —
    `sourceCompatibility`/`targetCompatibility` are pinned to 17 instead,
    since toolchain auto-download needs network access to a toolchain
    repository that isn't guaranteed available; whatever JDK invokes
    `./gradlew` compiles it directly.
  - `build.gradle.kts` gained: the `com.gradleup.shadow` plugin (`9.6.1`,
    matching the wrapper version — the actively-maintained fork of the
    stalled `com.github.johnrengelman.shadow`), `implementation("org.gradle:gradle-tooling-api:9.6.1")`
    (pulled from `repo.gradle.org/gradle/libs-releases`, since current
    `gradle-tooling-api` releases stopped publishing to Maven Central a
    while back — its Central metadata tops out at an old 7.x snapshot),
    and `runtimeOnly("org.slf4j:slf4j-nop:2.0.16")` (silences the Tooling
    API's "no SLF4J providers found" warning without pulling in a real
    logging backend). `tasks.shadowJar` sets `archiveClassifier = "all"`
    and the `Main-Class` manifest attribute — classified `-all` (rather
    than reusing the plain `jar` task's output filename) specifically so
    the two never collide on disk when both get built in the same
    checkout (`assemble`, and therefore `./gradlew build`, already runs
    both by default — the shadow plugin wires `shadowJar` into `assemble`
    on its own). Bundling `gradle-tooling-api` into the shaded jar never
    conflicts with the plugin/model classes' `compileOnly(gradleApi())`
    above: the two run in entirely separate JVM invocations (the target
    build's own classloader vs. this bridge's own `java -jar` client
    process), never on the same classpath at once.
  - `tasks.test` gained `jvmArgs("--add-opens", "java.base/java.lang=ALL-UNNAMED")`
    — `ProjectBuilder` (now used by real `JvmfastModelBuilderTest` cases)
    injects synthetic classes into its classloader reflectively at
    project-creation time, which JDK 17+'s module system blocks without
    this; a known, documented requirement for Gradle's own test fixtures
    on JDK 17+, not specific to this project.
  - Tests: `JvmfastModelBuilderTest` now covers a real empty-model case
    (`ProjectBuilder` project with no `java` plugin → one module, zero
    dependencies) and a real dependency-resolution case (`ProjectBuilder`
    project with `java` applied, `mavenCentral()`, an `implementation` and
    a `testImplementation` real coordinate — asserts the resolved model
    reports them under the right configurations with the right versions).
    The latter is a deliberate, narrow real-network exception (same
    category as `tests/cli_test.rs`'s real-Maven-Central console-jar
    download on the Rust side) — unavoidable since `buildAll`'s entire
    job is walking a *resolved* configuration, and this project's test
    suite already needs network to resolve its own JUnit dependencies to
    run at all. `MainTest` covers `Main.toJson`'s shape (including the
    empty-dependencies case and quote/backslash escaping) with no network
    needed. `JvmfastModelBuilderPluginTest` is unchanged from the
    skeleton. Verified end to end by hand too, outside the test suite:
    `java -jar build/libs/jvmfast-gradle-bridge-0.1.0-all.jar <dir>
    <init-script>` against a real throwaway Gradle 9.6.1 project produces
    exactly the expected JSON on a *pure* stdout stream (verified by
    redirecting stdout and stderr to separate files and validating the
    stdout file parses as JSON on its own).
- [`build.rs`](build.rs) — resolves the "how does
  `jvmfast-gradle-bridge.jar` reach the end user" distribution question
  (seção 10: "um helper JVM empacotado com o jvmfast"): every `cargo
  build` shells out to `gradle-bridge/gradlew shadowJar` first (not plain
  `jar` — the embedded jar doubles as the Tooling API client driver, which
  needs `gradle-tooling-api` actually present on the classpath at
  runtime, unlike the plugin/model classes; rerunning only when
  `gradle-bridge/src`, `build.gradle.kts`, or `settings.gradle.kts`
  change, via `cargo:rerun-if-changed`) and embeds the resulting
  `*-all.jar`'s bytes straight into the `jvmfast` binary with
  `include_bytes!` — no runtime download, no separate release-asset
  channel to maintain. This is a real, deliberate cost: `cargo build`
  (not just `cargo test`, which already needed a JDK for Fase 3's
  `javac`/`java`-shelling tests) now requires a JDK on `PATH` plus
  whatever network access Gradle's own wrapper bootstrap needs on first
  run — accepted in exchange for the bridge having zero runtime network
  dependency of its own. `.github/workflows/rust.yml` also triggers on
  `build.rs`/`gradle-bridge/**` changes now, since `cargo build` depends
  on that directory.
- `src/gradlebridge/` — `extract_bridge_jar(cache_root)` writes the
  embedded jar to `<cache_root>/artifacts/sha256/...` via the same
  `CacheStore::write_artifact` (seção 5.1: temp file → verify checksum →
  atomic rename) every other cached artifact already goes through, keyed
  by the embedded bytes' own SHA-256 — so a rebuilt bridge jar with
  different bytes never collides with a stale extracted copy, and
  extraction is idempotent/safe to call on every `jvmfast import-gradle`
  invocation. Now actually called, by `src/gradleimport/` below. Tested
  (`tests/gradlebridge.rs`) against the real embedded jar (checks the
  "PK" zip magic bytes and idempotent re-extraction).
- `src/gradleimport/` (new) — `import_gradle(project_dir, manifest_path,
  cache_root)`, the `import-gradle` counterpart to `crate::import::import_pom`.
  Refuses to run if `manifest_path` already exists
  (`GradleImportError::ManifestAlreadyExists`) or if `project_dir` has no
  `gradlew`/`gradlew.bat` (`GradlewNotFound` — the Tooling API still needs
  a real Gradle distribution to connect to, seção 10's own documented
  limitation; jvmfast only avoids needing to understand *which* version).
  Flow: `gradlebridge::extract_bridge_jar` gets the bridge jar onto disk,
  `initscript::write_init_script` writes a temporary Groovy init-script
  (`initscript { dependencies { classpath(files(...)) } }` +
  `allprojects { apply plugin: ... }`, one instance per invocation via a
  process-wide `AtomicU64` counter added to the filename — plain
  `process::id()` alone isn't unique enough across concurrent invocations
  in the same process, e.g. parallel `cargo test` threads, and collided in
  testing before the counter was added), then `java -jar <bridge_jar>
  <project_dir> <init_script>` runs as a real subprocess and its stdout is
  parsed as `model::BridgeModel` (serde, mirrors `Main.toJson`'s shape
  exactly) via `serde_json`. A non-zero exit becomes
  `GradleImportError::BridgeFailed { status, stderr }`; unparseable stdout
  becomes `InvalidBridgeOutput`. Mapping onto `project.toml` (reusing
  `crate::import::render_manifest`, now `pub` from `src/import/` for this
  reason): dependencies from `compileClasspath`/`runtimeClasspath` become
  `[dependencies]` (deduplicated by coordinate); whatever
  `testCompileClasspath` adds *beyond* that set becomes
  `[dev-dependencies]` (since `testImplementation` extends
  `implementation` in a typical Gradle build, `testCompileClasspath` is
  usually a superset — only the genuinely test-only additions belong in
  dev-dependencies); `java-version` is always defaulted to `"lts"` with a
  report note, since `JvmfastDependencyModel` doesn't expose Gradle's
  configured Java version yet; a project with no version set
  (`"unspecified"`) is defaulted to `"0.1.0"` with a note; a report note
  about seção 10's documented mediation-divergence risk
  (Gradle=highest-version-wins vs. jvm-fast=nearest-depth-wins, seção 6.2)
  is always included, unconditionally, per seção 10's explicit ask
  ("vale documentar isso explicitamente para o usuário"); a note about no
  `[repositories]` being generated (jvm-fast defaults to Maven Central) is
  always included too. A multi-module Gradle result (more than one
  `JvmfastModule`, not producible by the current single-project-only
  `buildAll` but defensively handled) imports only the first, with a note
  — Fase 5 scope.
- `src/cli/import.rs` — now wires both `jvmfast import-pom [pom] [-o
  path]` and `jvmfast import-gradle [project]` (`project` defaults to the
  CLI's own project root — mirrors `import-pom`'s `pom: None` defaulting
  to `<root>/pom.xml`). Both share a `format_summary` helper for the
  one-line-per-note report output.
- `src/cli/error.rs` — `CliError::Import(#[from] ImportError)` and the new
  `CliError::GradleImport(#[from] GradleImportError)`.
- Tested with fixture-only POMs for `import-pom` (`tests/fixtures/import/`,
  `tests/fixtures/poms/import_metadata.xml`) — no real network, no real
  Maven Central, consistent with every other parsing-layer test in this
  repo; a `full_pom.xml` fixture exercises every conversion path (plain/
  interpolated/BOM-managed/pinned-range dependency versions,
  provided-scope skip, unresolved-property skip, unresolved-range skip,
  exclusions, dev-dependencies, BOM import, multiple repositories,
  profiles/plugins detection) in one integration test
  (`tests/import.rs`), plus dedicated `tests/import_range.rs` for the
  range-translation boundary and `tests/cli_import.rs` for the CLI wiring
  (default path, explicit path, already-exists rejection, report
  formatting). `import-gradle`, by contrast, is tested against a real,
  committed Gradle project fixture
  (`tests/fixtures/gradle/simple-project/` — its own `settings.gradle.kts`/
  `build.gradle.kts` declaring a real `implementation`/`testImplementation`
  pair, plus its own committed `gradlew`/`gradle-wrapper.jar`, copied from
  `gradle-bridge/`'s own wrapper so both pin the same Gradle 9.6.1): this
  is a deliberate, narrow real-network/real-subprocess exception (same
  category as `build`/`run`/`test`'s real JDK and `test`'s real-Maven-
  Central console-jar download) — `import-gradle`'s entire mechanism *is*
  a real Tooling API connection, so there's no meaningful way to fake it
  without testing nothing real. `tests/gradleimport.rs` exercises
  `gradleimport::import_gradle` directly (generated manifest content,
  dependency/dev-dependency split, report notes, `ManifestAlreadyExists`,
  `GradlewNotFound`); `tests/cli_import_gradle.rs` exercises the CLI
  wiring layer (explicit `project` path, defaulted path, already-exists
  rejection) the same way `tests/cli_import.rs` does for `import-pom`.

**Fase 5 (workspace e multi-módulo, seção 12) — foundation started: real
multi-module loading works end to end, cross-module dependencies/build
ordering/incremental build don't exist yet**:

- `src/manifest/dto.rs` — `ProjectManifest` gained an optional `workspace:
  Option<WorkspaceSection>` field (`WorkspaceSection { members: Vec<String>
  }`, from `[workspace].members`). Absence of the whole `[workspace]` table
  — the common, pre-Fase-5 case — was already silently ignored by serde
  before this change (`ProjectManifest` has no `deny_unknown_fields`), so
  adding the field is purely additive; every existing single-module
  `project.toml` keeps parsing exactly as before.
- `manifest::parse_workspace_members(path)` — new, same precedent as
  `parse_repositories`/`parse_java_version` (reads the manifest
  independently rather than extending `parse_module`, since `Module`
  itself has no workspace concept — seção 3.1 still doesn't model
  workspace membership on the domain type, only `Workspace.modules: Vec<Module>`
  does). Returns an empty `Vec` when `[workspace]` is absent, not an error.
- `workspace::load_workspace` — no longer hardcodes `modules: vec![module]`.
  A new private `collect_manifest_entries(root)` reads the root manifest,
  asks it for `[workspace].members`, then reads each
  `<root>/<member>/project.toml` in declared order, returning
  `Vec<(PathBuf, String)>` (path + raw contents) for root-then-members —
  both `load_workspace` (parses each into a real `Module`) and
  `current_manifest_hash` (feeds every manifest's contents into
  `lockfile::compute_manifest_hash`, which was already generic over
  multiple manifests — designed for exactly this since Fase 1, per its own
  doc comment — but only ever called with one until now) share it, so the
  two can never drift out of sync on ordering (`compute_manifest_hash` is
  order-sensitive). A member manifest that doesn't exist on disk becomes a
  typed `WorkspaceLoadError::Io` naturally (no separate existence check
  needed); two modules (root or members) sharing the same `[project].name`
  become `WorkspaceLoadError::DuplicateModuleName` — real diagnostics
  (`VersionRequest.origin_module`, `LockedRequest.module`, `jvmfast why`)
  are keyed by module name, so a silent collision would corrupt them.
- **Nothing else needed to change** for `jvmfast install`/`build` to start
  operating on real multi-module workspaces — this is the payoff of the
  Fase 1-4 "operate on `&[Module]`/`workspace.modules`, never index `[0]`"
  discipline documented as binding since before Fase 5 existed.
  `resolve::resolve`, `graph::build_graph`, `mediation::mediate`,
  `lockfile::build_lockfile`, `build::build`, and `testing::run_tests` all
  already iterated every module correctly and needed zero code changes —
  proven by `tests/multi_module.rs`'s end-to-end test (`jvmfast install`
  resolves and downloads a distinct dependency declared by each of two
  real modules loaded from disk into one shared `project.lock`, with
  correct per-module `[[request]].module` provenance; `jvmfast build`
  compiles both into their own independent `target/classes` trees) against
  a real mock HTTP server, not a synthetic in-memory `Workspace`.
- Tests: `tests/workspace.rs` gained multi-module coverage
  (root+members loading, aggregate manifest hashing in order,
  backward-compat single-module behavior unchanged, duplicate-name
  rejection, missing-member-manifest rejection) alongside the pre-existing
  single-module tests, none of which needed to change.
- **Cross-module dependencies — implemented.** A module can now declare
  `[workspace-dependencies]` (a separate table from `[dependencies]`,
  deliberately — a workspace module reference has no version to
  resolve/mediate, so mixing it into the same table as real Maven
  coordinates would blur two different concepts sharing only TOML syntax;
  chosen over overloading `[dependencies]` with a `{ module = true }`
  value shape after weighing both). Syntax: `[workspace-dependencies]\ncore
  = true` — keyed by module *name*, `false` rejected as a typed
  `ManifestError::InvalidWorkspaceDependencyValue` (mirrors
  `DependencyValue::BomManaged` never accepting `false` either).
  `Module` gained `workspace_dependencies: Vec<String>`
  (`manifest::convert::to_module`, sorted alphabetically for determinism —
  `HashMap` iteration order isn't). `graph::build_graph` now does two
  passes over `modules` (module_roots must be fully populated before any
  module's dependencies are processed, since a workspace-dependency can
  reference a module declared *later* in the slice) and, for each
  `workspace_dependencies` entry, emits a real `EdgeKind::WorkspaceModule`
  edge between the two modules' synthetic root `NodeId`s — the enum
  variant that had existed, unconstructed, since before Fase 5 started. A
  reference to a nonexistent module name is
  `GraphError::UnknownWorkspaceModule`, not a silent no-op or a fallthrough
  to an ordinary (and wrong) Maven coordinate lookup.
- **`jvmfast tree`/`jvmfast why` render workspace-module edges.** Both
  `cli::tree::format_tree`/`cli::why::format_why` previously silently
  dropped any graph edge whose `to` wasn't a real `ResolvedNode` (the
  `let Some(node) = graph.nodes.get(...) else { continue }` guard that
  already protected against unknown `NodeId`s) — exactly what a
  `WorkspaceModule` edge's `to` is, since it points at another module's
  synthetic root, never a resolved artifact. Both now build a reverse
  `NodeId → module name` map from `module_roots` and render those edges as
  `"<name> (workspace module)"`, then keep recursing/tracing into that
  module's own children — so `jvmfast why` can now report a dependency
  reached transitively *through* a sibling module's declaration, not just
  ones a module lists directly.
- **Topological build ordering — implemented.** `build::order::module_order`
  (Kahn's algorithm over `Module.workspace_dependencies`) is a new,
  independent function — deliberately *not* reusing `graph::build_graph`'s
  traversal, since `build` never re-resolves or touches the network
  (seção 8's own rule) and topological order is a pure function of
  `workspace.modules` alone. Same validation duplicated on purpose (an
  unknown module name is `BuildError::UnknownWorkspaceModule`, the same
  category `graph::build_graph` already rejects at resolve time, just
  independently since `build` doesn't go through that code path); a cycle
  (A depends on B depends on A) is `BuildError::CyclicModuleDependency`
  with every module involved named, never a partial build that silently
  compiles half a cycle and calls it success.
- **Inter-module classpath — implemented.** `build::build` now compiles
  `workspace.modules` in `module_order`'s order (not declaration order)
  and, for each module, adds every `workspace_dependencies` entry's
  already-compiled `target/classes` directory to that module's `javac -cp`
  — on top of the shared external classpath every module already got.
  Verified end to end with real `javac`, not just unit-level plumbing:
  `tests/build.rs::build_compiles_a_module_against_a_workspace_dependencys_classes`
  has an `api` module whose source genuinely imports and calls a class
  `core` defines — it only compiles successfully if both the ordering and
  the classpath assembly are correct.
- **`testing::run_tests` reconciled with `build`'s explicit model —
  implemented.** Previously left as a known gap (an implicit
  "every-previously-processed-module's classes are visible" accumulator,
  ordered by `[workspace].members` declaration order, regardless of any
  actual declared dependency); now uses `build::module_order` the same way
  `build::build` does, and each module's *compile-time* classpath for
  `src/test/java` includes only its own `target/classes` plus each
  `workspace_dependencies` entry's — not everyone processed so far. The
  Console Launcher's *run-time* classpath (passed to `console::run`)
  deliberately keeps accumulating every module's production+test classes
  regardless of declared dependency, since JUnit needs to resolve classes
  at runtime independent of who declared what at compile time — only the
  compile-time scoping changed, not execution. Verified end to end against
  the real JUnit Platform Console Launcher, not just unit-level plumbing:
  `tests/cli_test.rs::test_compiles_a_modules_tests_against_a_workspace_dependency`
  has an "api" module's test class that calls a class only "core" defines,
  reachable only via `[workspace-dependencies].core = true` — it compiles
  and the real JUnit run passes.
- **Incremental build — implemented, at module granularity.** New
  `src/build/fingerprint.rs`: `compute_module_fingerprint` hashes (SHA-256,
  every file listing sorted first — filesystem iteration order isn't
  guaranteed) a module's source file *contents* (not timestamps, which lie
  across git checkouts/CI), its resource file contents, its full classpath
  (external + workspace-dependency `target/classes` paths), the `javac`
  binary path itself (so `jvmfast jdk use` switching JDKs invalidates
  correctly), and — for transitive invalidation — every declared
  `workspace_dependencies` entry's *own* fingerprint (not its file
  contents again; `build::build` already computed dependencies'
  fingerprints first, in topological order, so this reuses that instead of
  re-hashing). `build::build` compares this against
  `target/classes/.jvmfast-build-fingerprint` (written atomically —
  temp file → rename, same discipline as `cache::CacheStore` — and only
  ever *after* a successful compile+copy, never before, so an
  interrupted/failed build can't leave a false "up to date" marker behind)
  and skips `compile`/`copy_resources` entirely when they match
  (`ModuleBuildSummary.up_to_date = true`, `compiled_files`/
  `copied_resources` both `0` — an honest "nothing ran this time", not "the
  module has nothing"). Any uncertainty (`target/classes` missing, no
  stored fingerprint, unreadable fingerprint file) forces a rebuild — same
  "cache is never a source of truth, corruption is resolved by rebuilding"
  principle as `src/cache` itself. `jvmfast run`/`jvmfast test` inherit
  this for free (both call `crate::build::build` internally) — their old
  doc comments claiming "always recompiles, no incremental build in v1"
  are now updated to reflect that module-level skip applies, though
  *file*-level incremental compilation within a module that does need
  rebuilding still doesn't exist (same pre-existing Fase 3 gap, narrower
  in scope now, not the same claim). `cli::build`'s summary text changed
  from "N module(s) built" to "N module(s) rebuilt, M up to date" to
  surface this honestly. Tested thoroughly given the correctness stakes of
  a caching feature (`tests/build.rs`): a second identical build skips; a
  changed source or resource file triggers a rebuild; a manually deleted
  `target/classes` is treated as never-built; and — the case that would be
  easiest to get subtly wrong — a workspace dependency ("core") changing
  correctly invalidates its *dependent* ("api") even though `api`'s own
  files never changed, alongside a mirror test proving both stay up to
  date when truly nothing changed (so the transitive-invalidation test
  isn't just "always rebuilds everything" in disguise).
- **`[run]`/`[dev-dependencies]` per module — implemented via `--module`.**
  `jvmfast run --module <name>` and `jvmfast test --module <name>` (new
  flag on both) pick which module's `[run]`/`[dev-dependencies]` apply —
  any module, root or member, can declare its own. Omitting the flag
  resolves to the root module (`cli::context::resolve_target_module`,
  `None` → `workspace.modules[0]`, which `workspace::load_workspace`
  guarantees is always the root — the manifest entry it reads first,
  before any `[workspace].members`), so a workspace with no
  `[workspace]` table at all — the pre-Fase-5 case — behaves exactly as
  before, unchanged. For `test`, `--module` has a second effect beyond
  picking `[dev-dependencies]`: it also restricts *which* module's
  `src/test/java` gets compiled and run at all (`testing::run_tests`
  gained a `target_module: Option<&str>` parameter) — `jvmfast test`
  unscoped still compiles/runs every module's tests, same as before.
  An unknown `--module` name is a typed error
  (`CliError::ModuleNotFound`/`TestError::UnknownModule`), checked before
  any compilation starts. Verified end to end with real `javac`/JUnit:
  `tests/cli_run.rs` has a root and a member module each with a
  *different* `main-class`, proving `--module` selects the right one;
  `tests/cli_test.rs` has a root module with a genuinely *failing* test
  and a member with a passing one — the unscoped run fails (both run),
  `--module worker` only compiles/runs the member's test and passes,
  proving the restriction is real rather than "happened to pass anyway."
  `[repositories]`/`java-version` deliberately stay root-manifest-only —
  they read as whole-workspace configuration (which repository to
  resolve against, which JDK to use) rather than per-module concerns, so
  extending them per-module wasn't attempted.
- **No per-module diagnostics surface change.** `jvmfast why`/`jvmfast tree`
  already handled N modules correctly before this pass (`module_roots:
  HashMap<String, NodeId>`, proven by the pre-existing
  `tests/graph_construction.rs` multi-module test) — this pass didn't need
  to touch them, but they've also not been exercised end to end against a
  real multi-module `project.toml` on disk the way `tests/multi_module.rs`
  now does for `install`/`build`; worth a follow-up test, not a known bug.

**Known, deliberate gaps inside Fase 1** (typed errors, not silent
shortcuts):

- `jvmfast add <coord>` requires an explicit `@version` — "latest release"
  needs repository metadata (`maven-metadata.xml`) lookup, not built yet;
  rejected via `CliError::VersionOmittedNotSupported`.
- `jvmfast add --dev` is rejected (`CliError::DevDependenciesNotSupported`)
  — editing `[dev-dependencies]` from the CLI isn't implemented. Resolving
  dev-deps already declared directly in `project.toml` *is* implemented
  now (`manifest::parse_dev_module`, Fase 3/`jvmfast test`), so this gap
  is narrower than it used to be: it's specifically about `add`/`remove`
  never touching `[dev-dependencies]`, not about dev-deps being unusable.
- ~~`download::fetch_checksum` assumes every artifact publishes a
  `.sha256` sidecar~~ **Fixed.** Discovered this session testing
  `jvmfast test` against real Maven Central for the first time in this
  project (affected `install`/`add` equally, not Fase-3-specific): real
  Maven Central is inconsistent about `.sha256` — some artifacts publish
  it (the JUnit Console Standalone jar, luckily), many common ones don't
  and only publish `.sha1` (confirmed by hand against `slf4j-api`,
  `guava`, `hamcrest` — all 404 on `.sha256`, all 200 on `.sha1`).
  `DownloadClient::fetch_checksum` now returns a `PublishedChecksum`
  (`Sha256`/`Sha1`) and falls back from `.sha256` to `.sha1` on a 404;
  `DownloadClient::fetch_verify_and_cache`/`fetch_verify_and_cache_many`
  (new — replace the old `fetch_checksum`-then-build-`ArtifactRequest`
  pattern in `cli::install::resolve_downloads`,
  `testing::devdeps::resolve_dev_classpath`,
  `testing::console::ensure_console_jar`) verify against whichever
  algorithm was published, then compute and cache under the artifact's
  **real SHA-256** regardless (the cache/lockfile identity is always
  SHA-256, seção 5 — the published checksum is only ever used for
  download-integrity verification, never stored as-is). `ArtifactRequest`/
  `download_artifact`/`download_many` are unchanged and still used as
  before for `download_locked_packages` (an existing `project.lock`
  always already has the confirmed real SHA-256, no fallback needed
  there). One accepted, documented trade-off: artifacts with no
  `.sha256` sidecar lose the "skip download, already cached" fast path
  on the *first* resolve (the cache is indexed by SHA-256, unknown until
  after downloading+hashing when only `.sha1` is published) — but never
  again after that, since the real SHA-256 is what lands in
  `project.lock`. Verified against real Maven Central (`slf4j-api`
  installs and locks cleanly now, was a hard failure before).
- ~~`pom::xml` parses `<scope>` but `graph::build_graph` never filters on
  it~~ **Fixed.** Same discovery session, same real-Maven-Central testing:
  a dependency's `test`/`provided`/`system`-scoped children were being
  treated as ordinary transitive dependencies instead of excluded from
  propagation (real Maven semantics: only `compile`/`runtime` — and the
  unmarked default, which *means* `compile` — propagate transitively).
  Surfaced trying `org.assertj:assertj-core` as a real
  `[dev-dependencies]` entry, which pulled in a test-scoped
  `hamcrest-core` dependency with an unresolved `${hamcrestVersion}`
  property (a *different*, still-open, and much older gap — "no property
  interpolation/parent POM inheritance", documented in `pom::xml`'s own
  doc comment since Fase 1 — that this scope leak was incidentally
  triggering). `PomDependency` now carries the raw `<scope>` string
  (`pom::xml::on_close`); `graph::build_graph` skips
  `propagates_transitively(&transitive.scope) == false` children before
  enqueuing them. Verified against real Maven Central: `assertj-core` as
  a `[dev-dependencies]` entry now resolves and its `assertThat` API
  compiles/runs correctly in `jvmfast test` (was a hard failure before,
  via the property-interpolation gap it used to trigger as a side
  effect). Note `com.google.guava:guava` still fails as a *production*
  dependency — but via that same older, separate, already-documented
  property-interpolation/parent-inheritance gap (guava's own POM manages
  `jsr305`'s version through its own `<dependencyManagement>`, which this
  project's POM parser doesn't resolve), not the scope-filtering gap this
  entry describes, and not something either of this session's two fixes
  ever claimed to address.
- `jvmfast update <coord>` (targeted update) is rejected
  (`CliError::TargetedUpdateNotSupported`) — only a full re-resolution
  (`jvmfast update`, no coordinate) is implemented.
- `tree`/`why` re-resolve in memory rather than reconstructing purely from
  an existing `project.lock` — `Lockfile`/`LockedPackage` don't persist
  `mediation_reason`, so a lockfile-only reconstruction can't produce the
  same diagnostic `why` promises without a schema extension first.
- Only the `default` key of `[repositories]` is used, and only as a single
  URL — no multi-repository fallback trying each declared repo in order,
  and no per-repository-host download throttling (the doc mentions both;
  `ArtifactRequest` has no repository identity to throttle by yet).
- `graph::build_graph` still rejects `^`/`~` ranges reaching an actual
  dependency as `GraphError::UnresolvedVersionRange` — no "available
  versions" source exists to resolve them against.
- `cache::cache_root()` resolves `~/.cache/jvmfast/` via `$HOME` only
  (Unix); no cross-platform (`dirs` crate) support yet.

**Known, deliberate gaps inside Fase 2 so far**:

- `jvmfast jdk install <version>` only accepts a major version (e.g. `21`)
  — an exact pinned version (`21.0.2-tem`) would need the Adoptium
  `/v3/assets/version/{version}` endpoint, not implemented yet; rejected
  via `JdkError::ExactVersionNotSupported`.
- `jvmfast jdk list` only lists *installed* JDKs — listing what's
  *available* to install would mean enumerating every release per major
  version, not just latest; not implemented.
- `jvmfast jdk use` requires the target major version to already be
  installed (`CliError::JavaVersionNotInstalled` otherwise) — it never
  triggers an install itself, unlike `install`/`update`'s
  auto-install-with-confirmation behavior (seção 7) for manifest
  `java-version` resolution.
- `resolve_project_java_version`/`ensure_project_jdk` hardcode
  `cli::context::ADOPTIUM_API` (the real `api.adoptium.net`), same as
  `install_jdk` — there's no injection point to point them at a mock
  server from a *public*-API test, so (like `tests/cli_jdk.rs` already
  does for `install_jdk`) exercising them end to end means calling the
  lower-level `jdk::resolve_feature_version`/`jdk::install` directly
  against a mock, not `cli::install`/`cli::jdk::resolve_project_java_version`
  itself. Automated tests cover `install()`'s two call sites (fresh
  resolve vs. reused-lock) only via a pre-installed fake JDK directory, so
  neither path needs a real network call to pass.
- The interactive decline branch of `ensure_project_jdk` (stdin prompt,
  `CliError::JdkInstallDeclined`) has no automated test — blocking a test
  binary on real stdin is unsafe in this environment (could hang instead
  of failing fast), so that branch is exercised by code review/manual
  testing only, not `cargo test`.
- Windows isn't supported — `jdk::current_platform` only maps
  Linux/macOS × x86_64/aarch64, matching `cache::cache_root()`'s existing
  Unix-only stance.

**Known, deliberate gaps inside Fase 3 so far**:

- `[dev-dependencies]` resolved by `jvmfast test` are never persisted in
  `project.lock` — resolved and downloaded fresh on every `test` run,
  unlike production dependencies. `Lockfile` (seção 4) has no schema slot
  for a second resolved graph; adding one is a bigger, separate design
  task (same category of gap as the `"lts"` alias needed one in Fase 2).
- `jvmfast test --fail-fast` is rejected
  (`CliError::FailFastNotSupported`) — the JUnit Platform Console Launcher
  has no native stop-on-first-failure flag (confirmed against the real
  `--help` output of 1.14.4), so there's no faithful way to implement what
  seção 8.1 documents without either a fork-per-test-class hack or
  upstream JUnit support that doesn't exist; rejected rather than faked.
- JUnit Platform Console Standalone version is hardcoded
  (`testing::CONSOLE_VERSION = "1.14.4"`) — no way to override it from
  `project.toml`/CLI flag yet; not documented as configurable by seção 8.1
  either, so not clearly in-scope to add.
- `build`/`run`/`test` skip recompiling a whole module when its content
  fingerprint hasn't changed (`build::fingerprint`, added in Fase 5, seção
  12 — see that writeup) — but *within* a module that does need
  rebuilding, there's still no finer-grained incremental compilation
  (per-file source-hash skip, so `javac` always recompiles every source in
  a changed module, never just the files that actually changed inside
  it). Narrower gap than before Fase 5, not the same claim as "recompiles
  from scratch every call" used to be.
- `[project].source-encoding` (seção 3, parsed by
  `manifest::dto::ProjectSection` since Fase 1) isn't passed to `javac`
  yet (no `-encoding` flag) — `build` doesn't read it at all currently.
- Annotation processing relies entirely on `javac`'s automatic
  `META-INF/services` discovery (seção 8's documented v1 scope); no
  explicit `-processor`/`-Akey=value` support, and none planned before
  seção 8 says so.
- `BuildError::CompileFailed` surfaces raw `javac` stderr as-is — no
  structured per-diagnostic parsing (file/line/column), unlike the typed
  errors elsewhere in the codebase; seção 11's distinct
  compile-failure-vs-test-failure-vs-config-error exit codes aren't wired
  yet either (nothing in `cli::run`, the module, i.e. `src/cli/mod.rs`'s
  entrypoint function — not `src/cli/run.rs`, the new `jvmfast run`
  command, confusingly same name at different paths — maps `CliError`
  variants to distinct process exit codes today, for any command; it's
  always a flat `SUCCESS`/`FAILURE`).
- `jvmfast run` doesn't forward extra CLI arguments to the executed
  program (e.g. `jvmfast run -- foo bar`) — seção 8 doesn't mention this
  as required v1 behavior, and no `Command::Run` field exists for it yet.
- `run_main_class`/`compile` both use `std::process::Command` directly
  with no timeout — a hung user program or a hung `javac` blocks
  `jvmfast` indefinitely; acceptable for a local dev tool in v1, not
  flagged as a problem to fix without a concrete need.

**Known, deliberate gaps inside Fase 4 so far**:

- `import-gradle` only imports the *first* Gradle module it sees —
  `JvmfastModelBuilder.buildAll` only ever populates one `JvmfastModule`
  today (single-project only), and `src/gradleimport/` defensively handles
  (with a report note) a hypothetical multi-module result rather than
  erroring, but neither side actually walks Gradle subprojects yet —
  Fase 5 scope, same as jvm-fast's own multi-module support.
- `import-gradle`'s generated `project.toml` always defaults
  `java-version` to `"lts"` with a report note — `JvmfastDependencyModel`
  doesn't expose Gradle's configured Java version (toolchain or
  source/targetCompatibility) at all yet; extending the model to carry it
  (mirroring how `import-pom` reads `maven.compiler.release`/etc.) is a
  clean, self-contained follow-up, not started here.
- `import-gradle` never imports BOMs/exclusions/extra repositories the way
  `import-pom` does for Maven's `<dependencyManagement>`/`<exclusions>`/
  `<repositories>` — Gradle's own equivalents (platform/BOM dependencies,
  `exclude` blocks, custom `repositories {}`) aren't modeled by
  `JvmfastDependencyModel` at all; only the flattened, already-resolved
  dependency list per configuration is. `[repositories]` is always empty
  in the generated manifest (a report note says so), meaning resolution
  after import silently falls back to Maven Central
  (`cli::context::resolve_base_url`'s existing default) regardless of
  what repository the Gradle build actually used.
- `import-gradle` requires a real, working `gradlew`/`gradlew.bat` in the
  target project directory (`GradleImportError::GradlewNotFound`
  otherwise) — matches seção 10's own documented limitation ("exige que o
  projeto tenha um Gradle instalado ou um gradlew funcional"), not
  something jvm-fast tries to route around.
- `import-pom` never follows `<parent>` POM inheritance — a POM whose
  `<artifactId>`/`<version>`/properties/dependency versions come from a
  parent (extremely common in real multi-module Maven projects) fails
  fast (`MissingArtifactId`/`MissingVersion`) or silently skips affected
  dependencies (unresolved `${property}`) rather than resolving the
  parent chain. Same underlying gap `pom::xml` has documented since Fase
  1, just newly user-visible through a command that reads real-world POMs
  directly instead of only fixture-shaped ones.
- Maven version ranges without a single-pinned-value direct equivalent
  (`[1.5,)`, `(,2.0]`, `[1.0,2.0)`, multi-segment ranges) are always
  skipped with a report note, never resolved to "the greatest version
  satisfiable at import time" as seção 10 describes as the fallback —
  that needs a `maven-metadata.xml` lookup this codebase doesn't have yet
  (same missing piece as `jvmfast add` without a version and
  `GraphError::UnresolvedVersionRange`).
- A dependency with no `<version>` and no BOM import anywhere in
  `<dependencyManagement>` is skipped with a report note — `import-pom`
  doesn't attempt to resolve it against a *local* (non-import) managed
  entry with a plain version either (seção 10 only documents preserving
  BOM imports, not local `dependencyManagement` version pinning as a
  concept jvm-fast has anywhere).
- Repository entries beyond the first are imported into `[repositories]`
  as-is, but jvm-fast's own resolution only ever reads the `default` key
  (existing Fase 1 gap) — so an imported project with multiple POM
  repositories gets a manifest that *looks* complete but silently only
  resolves against the first one, same limitation `import-pom` itself
  reports as a note but can't fix on its own.
- `jvmfast import-pom` always writes to `project.toml` at the project
  root — no `-o`/output-path override wired to the CLI yet (the
  underlying `import_pom` function already takes an explicit
  `manifest_path`, so this is only a CLI-flag gap, not a design one).
- ~~No `jvmfast init` (seção 9.2) exists yet~~ **Fixed** — see the
  `jvmfast init` writeup below.

**`jvmfast init` (seção 9.2) — implemented**: `src/init/` (new module,
`init::init_project(project_dir, name, java_version) -> Result<InitReport,
InitError>`) writes a minimal `project.toml` (`[project]` with
`name`/`version = "0.1.0"`/`java-version`, an empty `[dependencies]`, and
— only when a `Main.java` placeholder is actually written, see below — a
`[run]` block pointing at it) plus `src/main/java`/`src/test/java`
directories. Refuses to run over an existing `project.toml`
(`InitError::ManifestAlreadyExists`) and, per seção 9.2 point 5, refuses
to run at all when a `pom.xml` is already present
(`InitError::PomXmlDetected`, pointing at `jvmfast import-pom` instead of
silently generating a from-scratch manifest that would drop the POM's
already-declared dependencies) — `import-gradle`'s `gradlew`-detection
gap is *not* mirrored here on purpose, since a bare `gradlew` with no
`pom.xml` is a much weaker signal of "this is already a real project" (a
fresh `jvmfast init` scaffold has no `gradlew` of its own to collide
with, unlike `pom.xml`, which `import-pom` reads directly). A
`Main.java` "Hello, World!" placeholder is written into `src/main/java`
only if that directory doesn't already contain any `.java` file
(recursively, `init::dir_has_java_files`) — re-running `init` after
manually deleting just `project.toml` (or pointing it at a directory that
already has real source under `src/main/java` for some other reason)
never clobbers it, and the generated manifest's `[run]` section is
omitted too in that case, since there's no `Main` class this `init`
invocation itself can vouch for.

One deliberate deviation from the doc's literal seção 9.2 text: `jvmfast
init` with no flags at all is documented as *interactive* ("pergunta nome
e java-version"). This implementation is **not** interactive — omitting
`--name`/`--java-version` derives sane non-interactive defaults instead
(`name` from the target directory's own name via
`Path::canonicalize().file_name()`; `java-version` defaults to `"lts"`,
the same alias every other `[project].java-version` consumer in this
codebase already resolves), and reports what it defaulted via
`InitReport.notes` (surfaced in the CLI summary) rather than staying
silent about it. This follows the same precedent
`cli::jdk::confirm_install`'s own doc comment already establishes for
this codebase: blocking a command on stdin is only ever used for an
*opt-out* confirmation with a `--yes` escape hatch (seção 7's JDK
auto-install prompt), never as the *only* way to supply a value a command
needs to proceed — a hung prompt in a non-terminal invocation (CI, or a
`cargo test` binary) is worse than a documented non-interactive default,
and the existing JDK-prompt precedent already accepts "the interactive
branch itself has no automated test, verified by code review/manual
testing only" as the trade-off for keeping stdin-blocking narrowly
scoped. `src/cli/init.rs` wires `jvmfast init [--name <name>]
[--java-version <version>]`, both optional. Tested with `tests/init.rs`
(the module function directly — explicit name/version, derived
defaults, placeholder skipped when `src/main/java` already has sources,
both refusal cases) and `tests/cli_init.rs` (the CLI wiring layer,
mirroring `tests/cli_import.rs`'s pattern); verified end to end by hand
too, outside the test suite, chaining a real `jvmfast init` → `install`
→ `build` → `run` against the real system `javac`/`java` and confirming
`Hello, World!` actually prints.

Next milestones, in order — **Fase 3 is complete** (build/run/test all
implemented), both real-world Maven Central gaps that testing surfaced
(checksum sidecar format, POM `<scope>` filtering) are fixed, and **Fase 4
is now complete**: `jvmfast import-pom` and `jvmfast import-gradle` are
both implemented end to end (the latter against a real Gradle Tooling API
connection, real dependency resolution, real Maven Central — see the
writeup and gaps above). The still-open, older, separately-documented
property-interpolation/parent-POM-inheritance gap (seção 3.3 area, present
since Fase 1) remains a real limitation for POMs that lean on it (e.g.
`com.google.guava:guava`'s own `<dependencyManagement>`-managed `jsr305`
dependency, and `import-pom`'s own parent-inheritance gap above) — worth
calling out as a candidate for a future pick, though not promised or
started here. **Fase 5 (workspace e multi-módulo) is complete**:
`workspace::load_workspace` really loads N modules from
`[workspace].members`; every downstream consumer (`install`, resolution,
mediation, the lockfile) already operated on them correctly with zero
core changes, proof that the Fase 1-4 "operate on `&[Module]`, never
index `[0]`" discipline paid off exactly as promised; and, on top of
that, cross-module dependencies (`[workspace-dependencies]`,
`EdgeKind::WorkspaceModule` is real code now, not a dead enum variant),
topological build ordering (`build::module_order`), and inter-module
classpaths in `jvmfast build` are all implemented and verified end to end
with real `javac` (see the Fase 5 writeup above) — and `jvmfast test`
(`testing::run_tests`) now agrees with `build` on what "sees another
module's classes" means (explicit `workspace_dependencies`-scoped
compile-time classpath, verified against the real JUnit Console Launcher)
instead of its own older, cruder, implicit accumulation. Build is also
incremental at module granularity now (`build::fingerprint`,
content-hash-based, transitive invalidation through workspace
dependencies verified explicitly in tests) — `build`/`run`/`test` all
skip a module whose inputs haven't changed. `jvmfast run`/`jvmfast test`
both gained `--module <name>` (`cli::context::resolve_target_module`) so
any module, root or member, can have its own `[run]`/`[dev-dependencies]`
— defaults to the root module when omitted, so single-module workspaces
are unaffected. `[repositories]`/`java-version` stay root-only by design
(whole-workspace configuration, not per-module concerns) — see the Fase 5
writeup above for the full breakdown. `jvmfast init` (seção 9.2) is also
now implemented — see its writeup above — so a `project.toml` can be
created from scratch without an existing Maven/Gradle project to import
from. Outside Fase 5, the natural next picks are, in no particular
priority order: credentials/auth (seção 3.2); global `config.toml`
loading beyond `[defaults]` (seção 3.5, overlaying the full documented
precedence chain onto `WorkspaceConfig::default()`); extending
`JvmfastDependencyModel` to carry Gradle's configured Java version
(closing `import-gradle`'s `java-version` gap above); or any of the other
Fase 1/Fase 2/Fase 3/Fase 4 gaps listed above, each independently
pickable.

**Multi-módulo (Fase 5) compatibility rules** — binding since before Fase 5
started, and now proven in practice by real multi-module loading and
cross-module dependencies (see the Fase 5 writeup above): resolution must
always operate on `Workspace.modules: Vec<Module>`, never a lone `Module`;
`VersionRequest.origin_module` and `LockedRequest.module` must always be
populated, even with only one possible value in a single-module workspace;
`GraphEdge`/`ResolvedNode` must never be merged into one struct;
`EdgeKind::WorkspaceModule` (now real, constructed by `graph::build_graph`
from `Module.workspace_dependencies`) must never be conflated with
`EdgeKind::External` — a workspace-module edge has no `ResolvedNode` on
its `to` side, ever, since there's no version to mediate for a sibling
module; CLI code must iterate `workspace.modules`, never index `[0]`.

## Core architectural model (from docs/architecture.md)

The single most important thing to internalize before touching design or
code here: **declaration and resolution are never the same struct.**

- `Module` declares dependencies (`project.toml`) — it never holds resolved
  versions.
- `Workspace` is the only thing that resolves (`project.lock`) — even in v1,
  which is single-module by *scope*, not by architectural limitation. The
  resolver always operates on `Workspace.modules` (a list), never on a lone
  `Module`, so multi-module (Fase 5 of the roadmap) requires no core rewrite.
- The dependency graph splits topology from resolution state on purpose:
  `GraphEdge` (who brought in what, `EdgeKind`) is pure topology;
  `ResolvedNode` (all requested versions + which one won + why,
  `MediationReason`) is resolution state. They connect only via `NodeId`,
  never a direct reference. This split is what lets `jvmfast why` reconstruct
  full diagnostic paths from `project.lock` alone, without re-fetching
  metadata.
- Version conflict mediation is a fixed-precedence chain, never competing
  heuristics: **nearest depth wins → higher version wins (tie-break) →
  deterministic tie-break (last resort)**. This deliberately differs from
  Gradle's default (highest-version-wins) — relevant when working on
  `import-gradle`, since a `jvmfast update` after import can select different
  versions than the original Gradle build did.
- `project.lock` must be sufficient, on its own, to explain any resolution
  decision (no re-fetching, no relying on an auxiliary cache as the only
  source of provenance). Any change to the graph/lockfile model must
  preserve this property.

Other decisions worth knowing before writing code in these areas:

- **BOMs** (seção 3.3) are resolved in a separate pass *before* the
  dependency graph — a coordinate→version table is built first, then used to
  fill in versions omitted in `[dependencies]` (signaled with `true`, never
  an empty string).
- **Exclusions** (seção 3.4) are applied during graph construction, before
  mediation — an excluded transitive never becomes a graph candidate.
- **Gradle import** (`jvmfast import-gradle`, seção 10) does not parse
  `build.gradle`/`.kts` statically. It uses the Gradle Tooling API through a
  bundled JVM helper (`jvmfast-gradle-bridge.jar`) that registers a custom
  `ToolingModelBuilder` via an init-script and returns a typed model — not
  stdout text parsing. This is the **one non-Rust component** in the stack;
  don't assume everything here is a Cargo crate.
- The cache (seção 5) is content-addressable (SHA-256-derived paths); writes
  go through `temp file → verify checksum → atomic rename`, and the cache is
  never treated as a source of truth — corruption is handled by rebuilding,
  never in-memory repair.

## Naming

- `jvm-fast` (hyphenated) — the project/repo/identity, used in prose.
- `jvmfast` (no hyphen) — the binary the user invokes, used only in command
  examples.
