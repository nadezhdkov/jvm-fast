# Benchmarks

All benchmarks were computed on [PLATFORM] using [JDK distribution/version] (for non-jvm-fast
tools), and come with a few important caveats:

- Benchmark performance may vary dramatically across different operating systems and
  filesystems. In particular, jvm-fast's cache strategy (content-addressable, seção 5 de
  `docs/architecture.md`) may benefit differently from reflinking versus hardlinking depending on
  the underlying filesystem.
- Benchmark performance may vary dramatically depending on the dependency tree being resolved.
  For example, a resolution that requires fetching a single large POM chain may appear similar
  across tools, since the bottleneck is network-bound rather than tool-agnostic.
- Maven and Gradle both use their own local caches (`~/.m2`, Gradle cache), which are not
  directly comparable to jvm-fast's content-addressable cache; "warm" here means each tool's own
  cache is populated, not that caches are shared across tools.

This document benchmarks against [REPRESENTATIVE PROJECT], as a representative example of a
real-world single-module Java project.

In each case, a smaller bar (i.e., lower) is better.

## Warm Installation

Benchmarking dependency installation (e.g., `jvmfast install`) with a warm cache. This is
equivalent to removing and recreating `target/` (or the Maven/Gradle equivalent) and repopulating
it with dependencies already downloaded once on the same machine.

<!-- ![install-warm](URL) -->

## Cold Installation

Benchmarking dependency installation (e.g., `jvmfast install`) with a cold cache. This is
equivalent to running `jvmfast install` on a new machine or in CI (assuming the cache is not
shared across runs).

<!-- ![install-cold](URL) -->

## Warm Resolution

Benchmarking dependency resolution (e.g., `jvmfast update`) with a warm cache, but no existing
`project.lock`. This is equivalent to deleting `project.lock` to regenerate it from `project.toml`.

<!-- ![resolve-warm](URL) -->

## Cold Resolution

Benchmarking dependency resolution (e.g., `jvmfast update`) with a cold cache. This is equivalent
to running `jvmfast update` on a new machine or in CI.

<!-- ![resolve-cold](URL) -->

## Build

Benchmarking `jvmfast build` against direct `javac` invocation and Maven/Gradle compile phases,
isolating the overhead jvm-fast adds on top of the compiler itself.

<!-- ![build](URL) -->

## Reproduction

<!-- TODO: fill in once scripts/benchmark exists for jvm-fast, following the shape of uv's
     scripts/benchmark package (wraps hyperfine to compare tools). -->

All benchmarks were generated using the `scripts/benchmark` package, which wraps
[`hyperfine`](https://github.com/sharkdp/hyperfine) to facilitate benchmarking jvm-fast against
Maven and Gradle.

The benchmark script itself has a few requirements:

- A local jvm-fast release build (`cargo build --release`).
- An installation of the production `jvmfast` binary in your path.
- The [`hyperfine`](https://github.com/sharkdp/hyperfine) command-line tool installed on your
  system.
- Local installations of Maven and Gradle for comparison.

To benchmark resolution against Maven and Gradle:

```shell
jvmfast run resolver \
    --jvmfast-project \
    --maven \
    --gradle \
    --benchmark resolve-warm --benchmark resolve-cold \
    --json \
    ../projects/example
```

To benchmark installation against Maven and Gradle:

```shell
jvmfast run resolver \
    --jvmfast-project \
    --maven \
    --gradle \
    --benchmark install-warm --benchmark install-cold \
    --json \
    ../projects/example
```

Both commands should be run from the `scripts/benchmark` directory.

After running the benchmark script, generate the corresponding graph via:

```shell
cargo run -p jvmfast-dev --all-features render-benchmarks resolve-warm.json --title "Warm Resolution"
cargo run -p jvmfast-dev --all-features render-benchmarks resolve-cold.json --title "Cold Resolution"
cargo run -p jvmfast-dev --all-features render-benchmarks install-warm.json --title "Warm Installation"
cargo run -p jvmfast-dev --all-features render-benchmarks install-cold.json --title "Cold Installation"
```

You need to install a font compatible with the rendering tooling if labels are missing in the
generated graph.

## Acknowledgements

The inclusion of this `BENCHMARKS.md` file was inspired by the benchmarking documentation used by
[uv](https://github.com/astral-sh/uv/blob/main/BENCHMARKS.md), the "uv for Python" project that
jvm-fast takes its overall philosophy from.

## Troubleshooting

### Flaky benchmarks

If you're seeing high variance when running the cold benchmarks, it's likely that you're running
into throttling or DDoS prevention from your ISP or from the Maven repository you're pulling
from. In that case, forceful TCP resets can occur when the same requests are made in a very short
time (especially true for `jvmfast`, which parallelizes downloads). A possible workaround is to
connect to a VPN to bypass filtering.
