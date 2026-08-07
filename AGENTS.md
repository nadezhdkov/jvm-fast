- Read CONTRIBUTING.md for guidelines on how to run tools
- ALWAYS ensure that new tests use the same style as existing tests for all parts of the test
- ALWAYS check whether the behavior of a new test is already covered by an existing test
- PREFER integration tests, e.g., at `tests/...` over unit tests (this project has no inline
  `#[cfg(test)]` modules anywhere — keep it that way)
- PREFER running specific tests over running the entire test suite, e.g., `cargo test <name>`
- PREFER plain `assert!`/`assert_eq!`/`matches!` over introducing a snapshot-testing dependency —
  this project does not use `insta` (or any snapshot-testing crate); do not add one without
  discussing it first
- NEVER perform builds with the release profile, unless asked or reproducing performance issues
- AVOID using `panic!`, `unreachable!`, `.unwrap()`, unsafe code, and clippy rule ignores
- PREFER patterns like `if let` to handle fallibility
- ALWAYS write `SAFETY` comments following our usual style when writing `unsafe` code
- PREFER `#[expect()]` over `[allow()]` if clippy must be disabled
- PREFER let chains (`if let` combined with `&&`) over nested `if let` statements
- NEVER update all dependencies in the lockfile and ALWAYS use `cargo update --precise` to make
  lockfile changes
- NEVER assume clippy warnings or test failures are pre-existing, it is very rare that `main` has
  warnings
- ALWAYS keep domain errors typed (`thiserror`), NEVER use `anyhow` or a generic `String` as an
  error type anywhere in `src/` — jvm-fast is a single binary crate (`lib.rs`/`main.rs` split,
  no Cargo workspace, no internal crates yet; see `docs/CONVENTIONS.md`), so this applies to
  every module, not just a subset
- PREFER `async`/`tokio` only where there is real concurrency (currently: `src/download`); NEVER
  introduce `async` in code that does not need it
- NEVER let `tests/build.rs`, `tests/cli_build.rs`, `tests/run.rs`, `tests/cli_run.rs`, or
  `tests/cli_test.rs` mock `javac`/`java` — these are the intentional exception to the
  no-real-network/no-real-toolchain rule, since they exist to prove the real compiler/JVM is
  invoked correctly
- NEVER let `tests/cli_test.rs` mock the JUnit Platform Console Standalone download — it is the
  one other intentional exception, since `jvmfast test` always fetches it from Maven Central for
  real
- PREFER fixtures (synthetic manifests/POMs) over real network access in every other test — see
  `docs/CONVENTIONS.md`
- ALWAYS keep `GraphEdge` (topology) and `ResolvedNode` (resolution state) as separate structs,
  never merge them
- ALWAYS keep `Module` (declared) and `Workspace` (resolved) as separate structs, never let one
  stand in for the other
- NEVER treat the cache as a source of truth — corruption is resolved by rebuilding, never by
  in-memory repair
- PREFER top-level imports over local imports or fully qualified names
- AVOID shortening variable names, e.g., use `version` instead of `ver`, and `dependencies`
  instead of `deps`
- PREFER [`TypeName`] references when writing Rust doc comments
