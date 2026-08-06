# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

**Fase 1 (docs/architecture.md seção 12 — "resolução e cache") is
complete.** The `jvmfast` binary resolves `project.toml`, downloads
artifacts over real HTTP, and writes/reads `project.lock`, end to end, via
`install`/`update`/`add`/`remove`/`tree`/`why`. Fase 2 (JDK management),
Fase 3 (build/run/test), and Fase 4 (interop) are **not started** —
`docs/architecture.md` seção 12 and the roadmap below are the spec to
implement against for those. See "Roadmap" below for the specific gaps left
inside Fase 1 itself (targeted `update <coord>`, `add` without an explicit
version, dev-dependencies, multi-repository fallback, per-host download
throttling) — each is a typed, rejected-not-faked error today, not silent
scope creep.

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
  in `tests/manifest_parsing.rs`, using fixtures under `tests/fixtures/`)
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

Next milestones, in order — **Fase 2** (JDK management, seção 7):
`jvmfast jdk install/list/use`, `java-version` resolution in the manifest
(including the `"lts"` alias-to-concrete-version-at-first-resolve rule) →
**Fase 3** (build/run/test, seção 8): `jvmfast build`/`run`/`test`,
`javac` compilation, JUnit Platform Console integration → credentials/auth
(seção 3.2) → global `config.toml` loading (seção 3.5, overrides
`WorkspaceConfig::default()`) → the gaps listed above, each independently
pickable.

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
