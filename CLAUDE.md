# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository state

**All five fases are implemented; the resolver is not yet correct against
real Maven Central.** Read `docs/architecture.md` **seção 16** before doing
any resolution work — it is the post-implementation review, and where it
disagrees with seções 1–15 (the pre-code spec), seção 16 wins.

The short version: the pipeline consumes the **raw** POM, but Maven resolves
against the **effective** POM (parent chain + `${property}` interpolation +
`<dependencyManagement>` + `<optional>` filtering). So
`com.fasterxml.jackson.core:jackson-databind:2.17.0` — the example in seção
3 of the architecture doc — does not resolve: its two compile dependencies
declare `<version>${jackson.version.core}</version>` and
`${jackson.version.annotations}`, both defined in the `jackson-base` parent,
so the resolver tries to fetch a URL containing a literal `${` and gets a
404. Guava fails differently (transitive deps with no `<version>` at all).
The 228 tests pass because the local fixtures use literal versions and no
`<parent>` — see seção 13.1's added note on why fixtures alone couldn't
catch this.

Four more of the same kind, all detailed in seção 16: Maven versions aren't
semver so mediation silently falls back to string compare (`"10.0" < "9.0"`,
so "higher version wins" picks the lower one); transitives of versions that
*lost* mediation stay in the graph and the classpath (Maven prunes them);
`type`/`classifier` aren't modeled, so classifier'd artifacts resolve to the
wrong URL; and POM fetching is sequential blocking I/O with no `poms/`
cache, which inverts the project's own performance premise (the parallel
part is JAR download, which the content-addressable cache already makes
free; the serial part is metadata, which dominates every cold resolve).

Treat "Fase N complete" below as **feature coverage, not Maven parity**. The
acceptance metric proposed in seção 16.8 is a single integration test:
resolve `spring-boot-starter-web:3.3.0` and require set-equality with
`mvn dependency:list`.

What *is* solid: the structural decisions all held up. Declaration/resolution
separation, `GraphEdge`/`ResolvedNode` separation, the self-sufficient
lockfile, content-addressable cache with atomic writes, typed errors, Tooling
API over stdout scraping. None of the seção 16 fixes require reopening them —
they fit in existing seams (a new `EffectivePom` stage before `build_graph`,
a version-ordering module replacing `SemVer` in comparison paths, an
expand/mediate/prune loop inside `build_graph`, a wider coordinate type).

With that framing, the five fases are implemented end to end, with real (not
mocked) integrations where the doc calls for them:

- **Fase 1** (seção 6/12, resolução e cache) — `install`/`update`/`add`/
  `remove`/`tree`/`why` resolve `project.toml`, download over real HTTP, and
  read/write `project.lock`.
- **Fase 2** (seção 7, JDK management) — `jdk install <major-or-exact>`/
  `jdk list [--available]`/`jdk use [--yes]` against the real Adoptium
  API, on Linux/macOS/Windows; `[project].java-version` (including
  `"lts"`) auto-installs and is pinned in `project.lock`; global
  `[network]`/`[output]` config is read from `~/.config/jvmfast/config.toml`.
- **Fase 3** (seção 8/8.1, build/run/test) — `build`/`run`/`test` compile
  with the real `javac`/`java` (source-encoding-aware, with a subprocess
  timeout, with configurable `[build]` annotation processors, with
  structured compile diagnostics), run tests via JUnit Platform Console
  Standalone (fetched as an internal tool dependency, never declared in
  `project.toml`, version overridable via `[testing].console-version`);
  `[dev-dependencies]` are lockfile-pinned across `test` runs; `run`
  forwards extra CLI args to the executed program; exit codes are
  distinct per failure category (seção 11).
- **Fase 4** (seção 10, interop) — `import-pom` reads `pom.xml` into
  `project.toml`; `import-gradle` drives a real Gradle Tooling API
  connection through a bundled JVM helper (`gradle-bridge/`, the one
  non-Rust component in the stack).
- **Fase 5** (seção 12, workspace) — `[workspace].members` loads real
  multi-module workspaces; `[workspace-dependencies]` wires cross-module
  edges, topological build order, inter-module classpaths, and
  module-granularity incremental builds; `run`/`test` take `--module`.
- **`jvmfast init`** (seção 9.2) — scaffolds a fresh `project.toml` +
  `src/main|test/java`, non-interactively (derives `name`/`java-version`
  defaults instead of prompting — see `src/init/`'s doc comment for why).

Two real-world Maven Central gaps surfaced by Fase 3 testing were fixed
along the way: `download::fetch_checksum` now falls back `.sha256` →
`.sha1` (most published artifacts only have `.sha1`), and
`graph::build_graph` now filters transitive POM dependencies by
`<scope>` (test/provided/system no longer leak in as compile-scoped).

See **"Known gaps"** below for what's deliberately not implemented (each
is a typed, rejected-not-faked error or a documented report note — never
silent scope creep), and **"v2 candidates"** for what's next.

- [`docs/architecture.md`](docs/architecture.md) — full architecture spec
  ("uv for Java"), source of truth for design decisions, numbered sections
  ("seção N"), written in Portuguese.
- [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) — coding/commit conventions
  + internal-crate README template (not yet applicable — single binary
  crate today).
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — human-contributor setup/testing
  guide (English); if it and `docs/CONVENTIONS.md` disagree,
  `docs/CONVENTIONS.md` + actual CI config win.
- [`STYLE.md`](STYLE.md) — prose/CLI-messaging style guide; **partially
  aspirational** (colored output, `--verbose`, hints — not implemented in
  `src/cli/` yet). Don't assume a behavior it describes exists without
  checking the code.
- `AGENTS.md` — coding-agent-specific conventions (test style, error
  typing, real-JDK/real-Maven-Central test exceptions); consistent with
  this file, kept separate since other tools read `AGENTS.md` by
  convention.

## Build, test, lint

- `cargo build` — build the `jvmfast` binary. **Requires a JDK on `PATH`
  and network access**: `build.rs` builds `gradle-bridge/` (`./gradlew
  shadowJar`) and embeds the jar via `include_bytes!`.
- `cargo test` — run all tests. **Requires a real `javac`/`java` on
  `PATH`** (`tests/build.rs`/`tests/cli_build.rs` shell out to the system
  JDK, not a mock; CI uses `actions/setup-java`).
- `cargo test <name>` — run a single test by name substring.
- `cargo clippy --all-targets -- -D warnings` — lint; CI fails on any
  warning, no silently-suppressed `#[allow(...)]`.
- `cargo fmt --all` — format; `-- --check` to verify without writing.

## Module map

- `src/domain/` — seção 3.1 types (`Module`, `Dependency`, `VersionReq`,
  `DependencyGraph`, `Lockfile`, `Workspace`, ...).
- `src/manifest/` — `project.toml` parsing (`parse_module`,
  `parse_repositories`, `parse_java_version`, `parse_workspace_members`,
  `parse_run_config`, `parse_dev_module`).
- `src/version/` — `SemVer`, `VersionRequirement` (exact/`^`/`~`).
- `src/bom/`, `src/exclusion/`, `src/pom/` — BOM table resolution,
  exclusion checks, `quick-xml` POM parsing (`PomProvider` trait).
- `src/graph/` + `src/mediation/` — candidate graph construction +
  mediation (`depth ASC → version DESC → tie-break`).
- `src/resolve/` — chains BOM → exclusions → graph → mediation.
- `src/lockfile/` + `src/workspace/` — manifest hashing, lock
  read/write/validate, multi-module workspace loading.
- `src/cache/` — content-addressable `CacheStore` (SHA-256 sharded,
  atomic writes) + `rusqlite` index.
- `src/maven/` — shared Maven repo layout helpers.
- `src/download/` — async (`tokio`+`reqwest`) `DownloadClient`
  (concurrency-capped downloads, checksum verify-and-cache).
- `src/jdk/` — Adoptium client, JDK install/list/find.
- `src/config/` — `~/.config/jvmfast/config.toml` `[defaults]`.
- `src/build/` — `javac` compile (`compile::CompileOptions`: encoding,
  annotation processors, timeout), structured diagnostic parsing
  (`diagnostics::parse_javac_diagnostics`), resource copy, workspace
  module ordering (`order::module_order`), classpath assembly,
  fingerprinting (`fingerprint::compute_module_fingerprint`) for
  incremental builds.
- `src/process/` — subprocess execution with a timeout
  (`status_with_timeout`/`output_with_timeout`), shared by `build::compile`
  and `testing::console::run`; deliberately not used by `run::launch`
  (the user's own executed program has no timeout, see Fase 3 below).
- `src/run/` — `java` subprocess execution, stdio inherited, forwards
  `program_args` after `main-class`.
- `src/testing/` — dev-dependency resolution (now lockfile-pinned via
  `devdeps::resolve_dev_classpath`, see Fase 3 below), JUnit Console
  Standalone fetch/run (`console::CONSOLE_VERSION`, overridable via
  `[testing].console-version`), `--filter` translation
  (`filter::glob_to_regex`).
- `src/import/` — `import-pom` (POM → `project.toml`, `range.rs` for
  Maven version ranges, `generate.rs` for manifest rendering).
- `src/gradlebridge/` + `src/gradleimport/` — embedded bridge jar
  extraction, init-script generation, `import-gradle` orchestration.
- `src/init/` — `jvmfast init` scaffolding.
- `src/cli/` — `clap` subcommands + orchestration for all of the above.
- `gradle-bridge/` — standalone Gradle project (own build/CI), the JVM
  side of `import-gradle`'s Tooling API bridge — see architecture note
  below.

## Known gaps (typed errors / report notes, not silent shortcuts)

**Fase 1** — no open gaps; the five items below were closed this session
(previous session had already closed the `add --dev`/multi-repository/
`dirs`-crate batch further down):
- `add <coord>` without `@version` now resolves "latest release" via
  `maven-metadata.xml` (`pom::HttpPomProvider::fetch_versions`,
  `cli::install::latest_release_version`) instead of always rejecting —
  `VersionOmittedNotSupported` is now only raised when the metadata
  lookup itself fails (network error, no repository has metadata, no
  published version parses as semver).
- `^`/`~` ranges now resolve against the same `fetch_versions` metadata
  (`graph::resolve_version_range`, `version::VersionRequirement::select_highest`)
  — the highest published version inside the range's bounds wins.
  `GraphError::UnresolvedVersionRange` still exists, but now only fires
  when metadata is unreachable/unsupported or genuinely no published
  version satisfies the range.
- `update <coordinate>` (targeted update) is wired up
  (`cli::install::update_targeted`): validates the coordinate is
  declared directly or already locked, then re-resolves. **Still
  scoped**: the re-resolution itself is the same full
  `install(force=true)` as plain `update` — there's no mechanism yet to
  pin every *other* coordinate at its currently-locked version while
  only this one is free to move (see the doc comment on
  `update_targeted` for why that's a materially bigger change, left for
  a future pass).
- `tree`/`why` no longer re-resolve over the network — both read
  `project.lock` directly via `lockfile::reconstruct_graph`, which
  rebuilds a `DependencyGraph`/`module_roots` purely from the lockfile
  (`LockedRequest`s at `depth == 1` → `ModuleDeclared` edges,
  `LockedPackage.dependencies` → `External` edges) plus the
  already-loaded `Module`s (`workspace_dependencies` → `WorkspaceModule`
  edges) — no provider, no HTTP. This needed a schema addition:
  `LockedPackage` now carries `mediation-reason`/`mediation-rejected`
  (`lockfile::flatten_mediation_reason`/`unflatten_mediation_reason`);
  both commands now require a present and valid `project.lock` (same
  `LockfileMissing`/`LockfileStale` errors `build` already used) instead
  of always working from a fresh in-memory resolve.
- Per-repository-host download concurrency throttling exists now
  (`download::DownloadClient`'s per-host `Semaphore` map, keyed by
  `host:port` parsed from each request URL — keying on host alone would
  collide two repositories served from the same hostname on different
  ports) — `NetworkConfig.per_host_concurrent` (default `4`) complements,
  never replaces, the existing global `concurrent_downloads` limit.

Closed a previous session: `add --dev`/`remove --dev` now edit `[dev-dependencies]`
directly (`cli::install::add_dev`/`remove_dev`, `cli::edit::add_dev_dependency`/
`remove_dev_dependency` — deliberately never call `install()`, since dev-deps
aren't part of the lockfile pipeline); `[repositories]` now tries every
declared URL in declaration order, not just `default`, for both POM fetch
(`pom::HttpPomProvider`, now `Vec<String>`-based) and JAR download
(`download::DownloadClient::fetch_verify_and_cache_from_any`/
`_many_from_any`) — `manifest::dto::ProjectManifest.repositories` is now an
`IndexMap` (not `HashMap`) specifically to preserve declaration order from
TOML, and `Lockfile`'s `resolved-from` is now per-package
(`lockfile::build_lockfile`'s `resolved_from` param is a
`HashMap<"coord@version", url>`, not one global string), since two packages
in the same resolution can legitimately come from different repositories;
`cache_root()`/`config::config_path()` now resolve via the `dirs` crate
(`dirs::cache_dir()`/`dirs::config_dir()`), honoring `$XDG_CACHE_HOME`/
`$XDG_CONFIG_HOME` on Linux and the native convention on macOS/Windows
instead of hardcoding `$HOME/.cache`/`$HOME/.config`.

**Fase 2** — no open gaps; the five items below were closed this session:
- `jdk install <version>` now accepts an exact pin (`"21.0.2"`/`"21.0.2+13"`),
  not just a major version — `jdk::parse_install_version_spec` picks
  `InstallVersionSpec::Major`/`Exact`, and `Exact` resolves via
  `AdoptiumClient::release_for_version` (`GET /v3/assets/version/{version}`),
  a separate endpoint from the `/v3/assets/latest/{feature}` one major
  versions use. `ExactVersionNotSupported`/`parse_major_version` still
  exist and still reject non-major specs — but only for the two places
  where a major version is genuinely the only sensible input:
  `[project].java-version` and `jdk use`.
- `jdk list --available` now also queries
  `AdoptiumClient::available_releases` (`GET /v3/info/available_releases`,
  `available_releases` field — the same endpoint `most_recent_lts` already
  used, just a different field) and shows every major version Adoptium
  currently distributes, not just installed ones.
- `jdk use <version> [--yes]` no longer requires the version to already be
  installed — a missing version now goes through the same
  confirm-or-`--yes` auto-install flow `jvmfast install`'s
  `[project].java-version` resolution already used
  (`cli::jdk::ensure_installed`, now parameterized by a per-caller prompt
  message instead of one hardcoded to "required by project.toml").
- `[network]`/`[output]` in `~/.config/jvmfast/config.toml` are read now —
  `NetworkConfig`/`OutputConfig`/`ColorMode` derive `Deserialize`
  (`#[serde(default)]` at the struct level, so partially-declared sections
  still fall back to the seção 3.5 defaults field-by-field), and
  `config::load_workspace_config` builds a full `WorkspaceConfig` from the
  global file that `workspace::load_workspace` now actually calls instead
  of hardcoding `WorkspaceConfig::default()`. **Still scoped**: this only
  wires the "config.toml global → defaults" step of the seção 3.5
  precedence chain — `project.toml` has nowhere to declare `[network]`/
  `[output]` per-project (not in `manifest::dto::ProjectManifest`), so
  steps 1-3 of the chain (CLI flags, env vars, project.toml) still don't
  exist; see the cross-cutting gap below.
- Windows is supported now: `jdk::current_platform` maps `"windows"` to
  Adoptium's `os` param; `jdk::install`/`extract_archive` picks `.zip`
  (via the new `zip` dependency) vs `.tar.gz` extraction from
  `release.filename`, never from the host OS, since Temurin always
  publishes `.zip` for Windows and `.tar.gz` elsewhere; and
  `jdk::javac_executable`/`java_executable` append
  `std::env::consts::EXE_SUFFIX` instead of the three hardcoded
  extension-less `bin/javac`/`bin/java` paths `cli::build`/`cli::run`/
  `cli::test` had before. Classpath joining (`std::env::join_paths`) and
  cache/config paths (`dirs` crate, closed in the Fase 1 pass above) were
  already cross-platform. **Not actually run on Windows** in this
  environment (Linux-only sandbox) — verified via fixture-built `.zip`
  archives and `EXE_SUFFIX`-based path assertions, never a real Windows
  JDK or `javac.exe` invocation.

**Fase 3** — two gaps left open on purpose (see below); the other seven
were closed this session:
- `[dev-dependencies]` are now lockfile-pinned (`Lockfile.dev_manifest_hash`/
  `dev_packages`/`dev_requests`, `lockfile::compute_dev_manifest_hash`/
  `is_dev_lockfile_valid`/`build_dev_packages`) — `testing::devdeps::resolve_dev_classpath`
  reuses them across `test` runs instead of re-resolving every time,
  re-resolving (and persisting back to `project.lock`) only when
  `[dev-dependencies]`/`[boms]`/`[exclusions]` actually changed. Separate
  hash from `manifest_hash` on purpose: `install`/`update` never resolve
  dev-deps (`cli::install::install` just carries the previous dev fields
  forward untouched), so reusing the same hash would make a stale
  `dev_packages` look valid after a prod-only `install`.
- JUnit Console Standalone version is overridable via
  `[testing].console-version` (`manifest::parse_console_version_override`)
  — `testing::CONSOLE_VERSION` is still the default when unset.
- `[project].source-encoding` is now passed to `javac -encoding` (was
  parsed but discarded before this pass).
- `[build].annotation-processors`/`[build.processor-args]` now map to
  `javac -processor`/`-Akey=value` — this **extends v1 scope** beyond
  what `docs/architecture.md` seção 8 originally specified ("configuração
  explícita de processor... fica fora da v1"); the doc was updated to
  match, explicit user decision, not scope creep. Automatic
  `META-INF/services` discovery still works unconfigured, as before.
- `CompileFailed` now carries structured diagnostics too
  (`build::CompileDiagnostic`/`parse_javac_diagnostics`, parsed from
  `javac`'s stable `file:line: error:`/`warning:` format) alongside the
  raw `stderr` (never dropped — diagnostics that don't match the pattern,
  e.g. flag-usage errors, only show up in `stderr`). Exit codes are now
  distinct per failure category end to end (`cli::exit_code_for`,
  docs/architecture.md seção 11's table: `2` resolution, `3` network,
  `5` compile, `6` test, `7` runtime) — `4` (auth) has no code path yet
  since jvm-fast has no credentials/auth (cross-cutting gap below).
- `run` forwards extra CLI args after `--` to the executed program
  (`jvmfast run -- <args>`, `run::run_main_class`'s new `program_args`
  param) — placed after `main_class`, never mixed with `[run].jvm-args`
  (which must precede it for `java` to parse them as JVM flags).
- `javac`/the JUnit Console Launcher now have a subprocess timeout
  (`process::status_with_timeout`/`output_with_timeout`, default 600s,
  `[build].timeout-secs`) — `jvmfast run`'s execution of the *user's own*
  `main-class` deliberately still has **no** timeout (that program may
  legitimately run forever, e.g. a server; killing it after N seconds
  would be wrong, not a safety net).

Left open, deliberately, after explicit confirmation:
- `test --fail-fast` still rejected (`FailFastNotSupported`) — the JUnit
  Platform Console Launcher has no native stop-on-first-failure flag; the
  only way to fake it would be streaming/regex-parsing the Console
  Launcher's human-readable tree output and killing the process on the
  first failure line, the exact kind of fragile-parsing-of-tool-output
  this project avoids everywhere else (typed errors, checksums instead of
  trust, structured Tooling API instead of `gradlew` stdout scraping).
- Incremental build is still module-granularity only, not per-file — true
  per-file incremental compilation needs a real cross-file dependency
  graph (which `.java` files reference which other files' public API) to
  be *correct*, not just fast; naively recompiling only changed-hash
  files risks silently stale `.class` output for unchanged files that
  depended on a since-changed public API. Treated as a from-scratch
  mini incremental-Java-compiler project, out of scope for this pass.

**Fase 4**
- `import-gradle` only imports the first Gradle module (no multi-project walk).
- `import-gradle` always defaults `java-version` to `"lts"` — model doesn't expose Gradle's configured version.
- `import-gradle` never imports BOMs/exclusions/extra repositories — `JvmfastDependencyModel` doesn't carry them.
- `import-pom` never follows `<parent>` POM inheritance (missing artifactId/version/properties fail or get skipped).
- Maven ranges without a single pinned value are always skipped with a note (needs `maven-metadata.xml`).
- A dependency with no `<version>` and no BOM import is skipped, not resolved against local `dependencyManagement`.
- Imported repositories beyond the first are written but never actually used in resolution (same Fase 1 gap).
- `import-pom` has no `-o` output-path CLI flag yet (the underlying function already takes one).

**Cross-cutting**
- No credentials/auth for private repositories (seção 3.2).
- Global `config.toml` precedence chain (CLI → env → project.toml → config.toml → defaults) still only has step 4 (config.toml → defaults) wired — see the Fase 2 note above. CLI flags/env vars for these settings don't exist yet, and `project.toml` has no `[network]`/`[output]` section to overlay even if they did.

**Correctness blockers (seção 16) — these are not "gaps", they are wrong
results.** Everything above this line is scope deliberately cut; everything
below is the implementation disagreeing with Maven. Do not open resolution
work without reading seção 16 first. Recommended order is technical, not by
effort — 16.1 first because almost nothing else is observable while most real
graphs fail to resolve at all, and 16.3 after 16.4 because parallelizing an
incorrect graph only reaches the wrong answer faster:

1. **16.1 — effective POM.** No `<parent>` chain, no `${property}`
   interpolation, no per-POM `<dependencyManagement>`, `<optional>` not even
   parsed. Needs a distinct `EffectivePom` type that `build_graph` accepts
   *instead of* `ParsedPom`, same typed discipline as `Module` vs
   `Workspace`. An unresolved `${...}` must be a typed error, never a fetch.
2. **16.2 — Maven version ordering.** `SemVer::parse` wants exactly three
   numeric components; `mediation::compare_versions` falls back to
   `str::cmp` otherwise, which is deterministically wrong. Needs Maven's
   `ComparableVersion` semantics. `SemVer` stays only for parsing the `^`/`~`
   authoring syntax, never for ordering repository versions. No ordering path
   may keep a `str::cmp` fallback.
3. **16.4 — graph pruning.** `expanded` is keyed `(coordinate, version)`, so
   both sides of a conflict expand and both subtrees reach the lockfile and
   classpath. Needs expand-level → mediate → prune → next level, which moves
   `mediate` inside `build_graph`. Data model (seção 3.1) is unchanged.
4. **16.3 — POM cache + parallel resolution.** Seção 5 specifies `poms/`
   (permanent TTL) and `metadata/` (24h TTL); neither exists, so every
   resolve re-fetches everything. Then make BFS level-parallel. Determinism
   survives because mediation is a pure function of the collected requests,
   but that becomes an invariant needing an explicit test rather than an
   accident of serial traversal.
5. **16.5 — `type`/`classifier` in the coordinate.** Changes the
   `project.lock` key format, so it is cheaper before there is any installed
   base; seção 15.4 already decouples lock-format version from binary
   version for exactly this.
6. **16.6 — documented-but-absent CLI surface.** No global `--json`,
   `--offline`, `--verbose`, `--quiet`, `--no-color`; no `cache`
   subcommand. `--offline` is how you'd verify 16.3 actually works.
7. **16.7 — snapshots.** Maven 3 defaults to unique snapshots, so the
   current "treat as a normal artifact" assumption 404s against a standard
   Nexus — the corporate case seção 3 advertises. Support it or reject it
   with a typed error; silence is the one option that's wrong.

## v2 candidates

**These come after the seção 16 correctness blockers above, not alongside
them.** Every item here adds surface to a resolver that still produces the
wrong dependency set on most real-world graphs; shipping more features on
that foundation makes the eventual fix more expensive, not less.

Natural next picks once the blockers are closed and the Fase 1-5 gaps are
triaged (no priority implied):

- Credentials/auth for private Maven repositories.
- Full `config.toml` precedence chain (CLI flags, env vars, per-project
  `[network]`/`[output]` in `project.toml`) — only the config.toml→defaults
  step is wired, see the Fase 2 note above.
- True targeted `update <coordinate>` (pin every other coordinate at its
  currently-locked version while only the given one is re-resolved) —
  `update_targeted` today validates the coordinate but still performs a
  full re-resolution, see the Fase 1 note above.
- `maven-metadata.xml`-backed open-ended range translation in
  `import-pom` (the `add`/`^`~` resolution use case is closed — see Fase
  1 above — but `import::range` still skips Maven ranges without a
  single pinned value).
- Multi-project Gradle import (`import-gradle` walking subprojects) —
  pairs naturally with jvm-fast's own multi-module support now that
  Fase 5 exists.
- Parent-POM inheritance for `import-pom` (property + version resolution
  through `<parent>`).
- Per-file incremental compilation within a module (needs a real
  cross-file dependency graph to stay correct — see the Fase 3 note
  above on why this wasn't attempted as a hash-based shortcut).
- `test --fail-fast` — only revisit if the JUnit Platform Console
  Launcher ever grows a native stop-on-first-failure flag; see the Fase 3
  note above on why the only current workaround (parsing its
  human-readable output) was rejected.

## Core architectural model (from docs/architecture.md)

The single most important thing to internalize before touching design or
code here: **declaration and resolution are never the same struct.**

- `Module` declares dependencies (`project.toml`) — never holds resolved
  versions.
- `Workspace` is the only thing that resolves (`project.lock`) — the
  resolver always operates on `Workspace.modules` (a list), never a lone
  `Module`. This is why Fase 5 (multi-module) needed no core rewrite.
- The dependency graph splits topology from resolution state:
  `GraphEdge` (who brought in what, `EdgeKind`) is pure topology;
  `ResolvedNode` (all requested versions + which won + why,
  `MediationReason`) is resolution state. They connect only via `NodeId`.
  This is what lets `jvmfast why` reconstruct full diagnostic paths from
  `project.lock` alone.
- Version conflict mediation is a fixed-precedence chain: **nearest depth
  wins → higher version wins (tie-break) → deterministic tie-break (last
  resort)**. Deliberately differs from Gradle's highest-version-wins —
  relevant to `import-gradle`, since a `jvmfast update` after import can
  select different versions than the original Gradle build did.
- `project.lock` must be sufficient, on its own, to explain any
  resolution decision — no re-fetching, no relying on an auxiliary cache
  as the only source of provenance. Any lockfile/graph model change must
  preserve this.
- **BOMs** (seção 3.3) resolve in a separate pass *before* the dependency
  graph — a coordinate→version table is built first, then fills in
  versions omitted in `[dependencies]` (signaled with `true`).
- **Exclusions** (seção 3.4) apply during graph construction, before
  mediation — an excluded transitive never becomes a graph candidate.
- **Gradle import** (seção 10) never statically parses `build.gradle(.kts)`
  — it uses the Gradle Tooling API through a bundled JVM helper
  (`jvmfast-gradle-bridge.jar`) registering a custom `ToolingModelBuilder`
  via an init-script, returning a typed model, never stdout text parsing.
  This is the one non-Rust component in the stack.
- The cache (seção 5) is content-addressable (SHA-256-derived paths);
  writes go `temp file → verify checksum → atomic rename`; the cache is
  never a source of truth — corruption is resolved by rebuilding, never
  in-memory repair.

**Multi-módulo (Fase 5) compatibility rules** — binding, proven in
practice: resolution must always operate on `Workspace.modules:
Vec<Module>`, never a lone `Module`; `VersionRequest.origin_module` and
`LockedRequest.module` must always be populated; `GraphEdge`/
`ResolvedNode` must never be merged into one struct; `EdgeKind::WorkspaceModule`
must never be conflated with `EdgeKind::External` — a workspace-module
edge has no `ResolvedNode` on its `to` side, ever; CLI code must iterate
`workspace.modules`, never index `[0]`.

## Naming

- `jvm-fast` (hyphenated) — the project/repo/identity, used in prose.
- `jvmfast` (no hyphen) — the binary the user invokes, used only in
  command examples.
