# Contributing

## Finding ways to help

Issues that are a good opportunity for contribution are labeled
[`help wanted`](https://github.com/nadezhdkov/jvm-fast/issues?q=is%3Aopen+is%3Aissue+label%3A%22help+wanted%22).
These require varying levels of experience with Rust and jvm-fast. Often these are tasks the
maintainers want to accomplish but do not currently have the resources to do themselves.

You don't need permission to start on an issue labeled as appropriate for community contribution.
However, it's a good idea to indicate that you are going to work on an issue to avoid concurrent
attempts to solve the same problem.

Please check in before starting work on an issue that has not been labeled as appropriate for
community contribution. Contributions are welcome for other issues too, but it's important to
make sure there's consensus on the solution to the problem first.

Outside of issues with the labels above, issues labeled
[`bug`](https://github.com/nadezhdkov/jvm-fast/issues?q=is%3Aopen+is%3Aissue+label%3A%22bug%22)
are the best candidates for contribution. In contrast, issues labeled `needs-decision` or
`needs-design` are _not_ good candidates for contribution — please do not open pull requests for
issues with these labels.

Please do not open pull requests for new features without prior discussion, particularly for
anything outside the current phase's scope (see the roadmap in
[`README.md`](README.md#status-do-projeto) and the explicit out-of-scope list in
[`docs/architecture.md#1`](docs/architecture.md)). Multi-module workspace support, third-party
plugins, shaded-jar packaging, repository publishing, and JPMS are all out of scope for v1 —
pull requests implementing them will be closed.

## Use of AI

<!-- TODO: link an AI usage policy once one exists, following the pattern astral-sh/uv points to
     astral-sh/.github's AI_POLICY.md. -->

Contributions assisted by AI tools are welcome as long as the author understands and can explain
every change. Contributions that do not follow the project's AI policy will be closed.

## Setup

[Rust](https://rustup.rs/) (stable toolchain) is required to build jvm-fast. A real JDK installed
and on the `PATH` (`javac`/`java`, any JDK 17+) is also required — from Phase 3 onward for
`cargo test` (`tests/build.rs`/`tests/cli_build.rs`/`tests/run.rs`/`tests/cli_run.rs`/
`tests/cli_test.rs` invoke the real compiler and JVM rather than a mock), and since Fase 4, for
`cargo build` itself too: [`build.rs`](build.rs) builds [`gradle-bridge/`](gradle-bridge/) (a real
Gradle project) and embeds the resulting jar into the `jvmfast` binary, which needs a JDK to run
`./gradlew` — see the `gradle-bridge` section below.

```shell
git clone https://github.com/nadezhdkov/jvm-fast.git
cd jvm-fast
cargo build
cargo test
```

## Testing

```shell
cargo test
```

To run a specific test by name substring:

```shell
cargo test <name>
```

[nextest](https://nexte.st/) works as a drop-in alternative (`cargo nextest run`) if you have it
installed, but it is not required and CI does not use it — `rust.yml` runs plain `cargo test`.

jvm-fast does not use snapshot testing (`insta` or otherwise) — assertions are plain
`assert!`/`assert_eq!`/`matches!`, following the style of the tests already in `tests/`.

### Fixtures instead of real network access

Tests use fixtures (synthetic manifests and POMs) — never real network access, even for the
resolver — with two intentional exceptions:

- `tests/build.rs`, `tests/cli_build.rs`, `tests/run.rs`, `tests/cli_run.rs`, and
  `tests/cli_test.rs` (Phase 3) run against the real JDK in the environment
  (`javac`/`java` on the `PATH`), since `jvmfast build`/`run`/`test` only invoke a real
  compiler/JVM — there is nothing meaningful to mock.
- `tests/cli_test.rs` also downloads the JUnit Platform Console Standalone from the real Maven
  Central on purpose, since it is the internal dependency `jvmfast test` always fetches from
  there, never from the project's own configured repositories.

See [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) for the full testing conventions.

### Local testing

Invoke a development build of jvm-fast with `cargo run -- <args>`. For example:

```shell
cargo run -- install
cargo run -- jdk list
```

## Formatting

```shell
# Rust
cargo fmt --all
```

Markdown is linted, not auto-formatted — see [Linting](#linting) below.

## Linting

```shell
# Rust — the exact command CI (rust.yml) runs
cargo clippy --all-targets -- -D warnings

# Markdown — the exact check CI (docs.yml) runs, config in .markdownlint.yml
npx markdownlint-cli2 "**/*.md"
```

jvm-fast has no Cargo features and no Cargo workspace (see
[Crate structure](#crate-structure) below), so `--all-features`/`--workspace` don't apply here.

`cargo-shear` (unused-dependency check) and `typos` (spell checking) aren't part of this
project's toolchain yet — CI only runs `cargo fmt --all -- --check`, `cargo clippy`, `cargo
build`, and `cargo test` (see `.github/workflows/rust.yml`). Feel free to run them locally, but
don't expect CI to enforce them.

### Windows support

jvm-fast is Unix-only today — `cache::cache_root()` resolves via `$HOME` and `jdk::current_platform`
only maps Linux/macOS × x86_64/aarch64 (see the Fase 1/Fase 2 gaps in [`CLAUDE.md`](CLAUDE.md)).
There is no Windows CI target and no `cargo-xwin` setup yet; contributions adding real Windows
support (not just compiling clippy for the target) are welcome, but should start from those two
functions.

## Crate structure

jvm-fast is a **single binary crate** today (`lib.rs`/`main.rs` split, no Cargo workspace, no
internal crates) — the "crate structure" question doesn't apply yet.
[`docs/CONVENTIONS.md`](docs/CONVENTIONS.md#template-de-readme-crates-internas) has a template
ready for if/when the project splits into a multi-crate workspace, but that split isn't decided,
so `cargo-depgraph` and friends have nothing to visualize right now.

### gradle-bridge

[`gradle-bridge/`](gradle-bridge/) is the one non-Rust, non-Cargo component in the repo — a
standalone Gradle project (own `build.gradle.kts`, own `gradlew`, own CI job) backing `jvmfast
import-gradle` (see [`gradle-bridge/README.md`](gradle-bridge/README.md) and CLAUDE.md's Fase 4
writeup). Unlike a typical "separate subproject," `cargo build` at the repo root *does* touch
it now: [`build.rs`](build.rs) shells out to `./gradlew shadowJar` here and embeds the result
into the `jvmfast` binary, so a JDK on `PATH` is required for `cargo build` itself, not just
`cargo test`. To build/test `gradle-bridge/` on its own (its own test suite, independent of the
Rust side):

```shell
cd gradle-bridge
./gradlew build
```

Requires a JDK on `PATH` (any JDK 17+), same as the Fase 3 test suites above.

## Domain conventions

- Domain errors are typed (`thiserror`), never `anyhow` or a generic `String`.
- `async` is used only where there is real concurrency — currently just `src/download`.
- `rustfmt`/`clippy` run clean, with no unexplained `#[allow(...)]`.
- `Module` (declared) and `Workspace` (resolved) are never the same struct.
- `GraphEdge` (topology) and `ResolvedNode` (resolution state) are never merged.

See [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) for the complete list of code and commit
conventions.

## Documentation

To preview any changes to the documentation locally:

1. Install the [Rust toolchain](https://www.rust-lang.org/tools/install).
2. Install [Node](https://nodejs.org/en/download) — needed to run `markdownlint-cli2` (see
   [Linting](#linting)).
3. Update [`docs/architecture.md`](docs/architecture.md) directly for design changes — it is the
   living specification and source of truth, not a description of what already exists.
4. Update [`CLAUDE.md`](CLAUDE.md) to reflect the current implementation state and next
   milestones whenever a milestone's status changes.
5. Follow [`STYLE.md`](STYLE.md) for prose/terminology conventions in docs and CLI-facing
   messages — note some of it (colored output, `--verbose`/`RUST_LOG` logging levels, hints) is
   written ahead of the implementation, same spirit as `docs/architecture.md` itself; check the
   current code before assuming a described behavior already exists.

After making changes to the documentation, run `npx markdownlint-cli2 "**/*.md"` (see
[Linting](#linting)) — this is what CI actually checks.

## Profiling and Benchmarking

jvm-fast has no benchmark suite yet — no `scripts/benchmark` package, no `jvmfast-dev` crate, no
CI benchmarking job. [`docs/templates/BENCHMARKS.md`](docs/templates/BENCHMARKS.md) is a
reference for the format a future `BENCHMARKS.md` should follow (adapted from
[uv's](https://github.com/astral-sh/uv/blob/main/BENCHMARKS.md)), not a document describing
anything that exists today.

### Logging

jvm-fast doesn't have a logging framework wired up yet (no `tracing`/`log`/`env_logger`
dependency) — `RUST_LOG`-driven trace logging is aspirational, described in
[`STYLE.md`](STYLE.md#logging) as target behavior, not something you can use today.

## Releases

<!-- TODO: fill in once a release process exists; jvm-fast has not published a release yet
     (see README.md#versionamento). -->

There is no release process yet. This section will describe changelog automation, version
bumping, and the release workflow once jvm-fast reaches its first tagged release, following the
shape of uv's release process (`scripts/release.sh` → editorialize `CHANGELOG.md` → PR → tag →
release workflow).
