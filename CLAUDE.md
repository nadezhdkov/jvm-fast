# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

This repository has a bootstrapped Rust CLI skeleton (`Cargo.toml`, `src/`)
implementing the domain model (seção 3.1) and `project.toml` manifest
parsing into `Module`. Resolution (version ranges, BOMs, exclusions,
transitive graph, mediation), lockfile I/O, the content-addressable cache,
HTTP download, and CLI subcommands (`install`/`add`/`remove`/`update`/
`tree`/`why`) are **not yet implemented** — `docs/architecture.md` seção 12
and the roadmap below are the spec to implement against for those.

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

Implemented: domain types (`src/domain/`: `Module`, `Dependency`,
`VersionReq`, `BomReference`, `DependencyGraph`, `GraphEdge`, `ResolvedNode`,
`Lockfile`, `Workspace`/`WorkspaceConfig` — the latter three are declared per
seção 3.1/3.5/4 but have **no constructor yet**, since they need lockfile
I/O / global config parsing that don't exist yet); manifest parsing
(`src/manifest/`: `parse_module(path) -> Result<Module, ManifestError>`);
version range parsing (`src/version/`: `SemVer`, `VersionRequirement::parse`
handles exact/`^`/`~` per seção 6.1, including the pre-release-exclusion
rule — not yet wired into the manifest/resolver, since there's no "available
versions" source to filter against until POM/metadata fetching exists);
BOM resolution (`src/bom/`: `resolve_boms` builds the `coordinate → version`
table per seção 3.3 — first-BOM-wins, first-entry-wins, transitive import
with the depth-10 limit — behind a `PomProvider` trait so the table-building
logic is testable with in-memory fixture POMs; no real POM fetching wired in
yet, and not yet wired into manifest/resolver either); exclusions
(`src/exclusion/`: `merge_exclusions` combines `Module.exclusions` across
`&[Module]` into one `coordinate → excluded-set` table, `is_excluded` checks
a parent/candidate edge against it per seção 3.4 — no wildcard support, by
design); real POM parsing (`src/pom/`: `ParsedPom`/`PomDependency`/
`ManagedDependencyEntry`/`PomProvider` — the shared abstraction `bom` and
`graph` both fetch through — plus `parse_pom_xml`, a real `quick-xml`
event-driven parser for `<dependencies>` and `<dependencyManagement>`; no
`${property}` interpolation, no `<parent>` inheritance, by design); graph
construction (`src/graph/`: `build_graph` walks each module's declared
dependencies and their transitives via `PomProvider`, applying
`exclusion::is_excluded` before a transitive becomes a candidate and
resolving `VersionReq::BomManaged` against the BOM table — produces a
`CandidateGraph` with **all** requested versions per coordinate, deliberately
*not* the doc's `ResolvedNode`/`DependencyGraph`, since those require
`selected`/`mediation_reason` that only mediation, the next milestone, can
honestly produce; a `^`/`~` range reaching a dependency here is a typed
`GraphError::UnresolvedVersionRange`, not silently treated as literal).

Next milestones, in order (each independently pickable in a future session):
mediation algorithm (seção 6.2 passo 5, tested against the seção 13.1 table
— consumes `graph::CandidateGraph` and produces the real
`domain::DependencyGraph`/`ResolvedNode`) → lockfile read/write +
manifest-hash (seção 4, first real `Workspace` constructor) →
content-addressable cache + SQLite index (seção 5) → parallel download via
reqwest/tokio (first `async` code in the codebase, also where `build_graph`
gets a real HTTP-backed `PomProvider`) → CLI command wiring for
`install`/`add`/`remove`/`update`/`tree`/`why` → credentials/auth (seção
3.2) → global `config.toml` loading (seção 3.5) → resolving `^`/`~` ranges
against real repository metadata (fills the `UnresolvedVersionRange` gap
above).

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
