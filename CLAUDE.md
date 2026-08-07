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
re-resolve on every build. **Fase 3 (build/run/test, seção 8) is in
progress**: `jvmfast build` compiles `src/main/java` with `javac` from the
project's resolved JDK and copies `src/main/resources` into
`target/classes`, end to end against a real (system) JDK in tests — it
requires a valid `project.lock` and never resolves/downloads/installs
anything itself (typed errors point at `install`/`jdk install` instead).
`run`/`test` are not started yet. Fase 4 (interop) is **not started** —
`docs/architecture.md` seção 12 and the roadmap below are the spec to
implement against for those. See "Roadmap" below for the specific gaps
left inside Fase 1 (targeted `update <coord>`, `add` without an explicit
version, dev-dependencies, multi-repository fallback, per-host download
throttling), Fase 2 (exact-version JDK install, listing *available* (not
just installed) JDKs, and global `config.toml` beyond `[defaults]`), and
Fase 3 (see below) — each is a typed, rejected-not-faked error today, not
silent scope creep.

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

## Build, test, lint

- `cargo build` — build the `jvmfast` binary
- `cargo test` — run all tests (integration tests for manifest parsing live
  in `tests/manifest_parsing.rs`, using fixtures under `tests/fixtures/`).
  **Requires a real `javac`/`java` on `PATH`** since `tests/build.rs`/
  `tests/cli_build.rs` (Fase 3) shell out to the system JDK, not a mock —
  the only test suite in this repo with that dependency; CI installs one
  via `actions/setup-java` (see `.github/workflows/rust.yml`)
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

**Fase 3 (build/run/test, seção 8) — in progress**:

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

**Known, deliberate gaps inside Fase 1** (typed errors, not silent
shortcuts):

- `jvmfast add <coord>` requires an explicit `@version` — "latest release"
  needs repository metadata (`maven-metadata.xml`) lookup, not built yet;
  rejected via `CliError::VersionOmittedNotSupported`.
- `jvmfast add --dev` is rejected (`CliError::DevDependenciesNotSupported`)
  — `[dev-dependencies]` parses in `ProjectManifest` but was never threaded
  into `Module` (no field for it, by the seção 3.1 struct as documented).
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

- `jvmfast run`/`jvmfast test` don't exist yet — `build` is the only
  seção 8 command implemented; `run` (compile-if-needed + execute
  `[run].main-class`) and `test` (seção 8.1: `src/test/java`,
  `[dev-dependencies]` on a test-only classpath, JUnit Platform Console
  Standalone) are next.
- `build` recompiles from scratch every call — no incremental
  compilation (source-hash/timestamp skip), matching seção 8's explicit
  "not mandatory in v1" scope note.
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
  yet either (nothing in `cli::run` maps `CliError` variants to distinct
  process exit codes today, for any command).

Next milestones, in order — rest of **Fase 3** (seção 8): `jvmfast run`
(compile if needed, then execute `[run].main-class`/`jvm-args`) →
`jvmfast test` (seção 8.1: `src/test/java` + `[dev-dependencies]` on a
test-only classpath, JUnit Platform Console Standalone as an internal
dependency, `--filter`/`--fail-fast`/`--report-xml`) → credentials/auth
(seção 3.2) → global `config.toml` loading (seção 3.5, overrides
`WorkspaceConfig::default()`) → the Fase 1/Fase 2/Fase 3 gaps listed
above, each independently pickable.

**Multi-módulo (Fase 5) compatibility rules** — already binding, not just
future work: resolution must always operate on `Workspace.modules: Vec<Module>`,
never a lone `Module`; `VersionRequest.origin_module` and
`LockedRequest.module` must always be populated, even with only one possible
value today; `GraphEdge`/`ResolvedNode` must never be merged into one struct;
`EdgeKind::WorkspaceModule` stays declared even while unreachable in
single-module; CLI code must iterate `workspace.modules`, never index `[0]`.

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
