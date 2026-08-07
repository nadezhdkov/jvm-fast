# jvmfast-gradle-bridge

The one non-Rust component in the jvm-fast stack
([`docs/architecture.md`](../docs/architecture.md) seção 10, "Migração de
projetos Gradle") — a standalone Gradle project, built and versioned
separately from the Cargo build that produces the `jvmfast` binary. See
[`CLAUDE.md`](../CLAUDE.md) for what's implemented here versus still
planned.

## Why this exists

`jvmfast import-gradle` doesn't parse `build.gradle`/`.gradle.kts`
statically — Groovy/Kotlin build scripts are full programming languages,
and correctly parsing an arbitrary one is a problem Gradle itself doesn't
solve without executing it. Instead, jvmfast uses the real Gradle Tooling
API to let the target project's own Gradle build resolve itself, then
reads back a typed model — never `gradlew` stdout text parsing.

That means a small piece of this feature has to be JVM code, and this jar
plays both roles of the Tooling API exchange:

- **Server side** (runs inside the *target* Gradle build's own
  classloader): a plugin (`JvmfastModelBuilderPlugin`) applied via a
  temporary init-script jvmfast generates, registering a
  `ToolingModelBuilder` (`JvmfastModelBuilder`) that walks
  `project.getConfigurations()` and reports a typed model
  (`JvmfastDependencyModel`/`JvmfastModule`/`JvmfastDependency`).
- **Client side** (runs as a plain `java -jar` process, invoked by
  jvmfast's Rust driver, `src/gradleimport/`): `Main` opens a real
  `GradleConnector` connection to the target project, requests that
  model with the generated init-script applied, and prints it as JSON to
  its own stdout — never `gradlew`'s, so there's no stdout-text-parsing
  fragility to a Gradle version bump.

## Building and testing

```shell
./gradlew build
```

Requires a JDK on `PATH` (any JDK 17+ works — `build.gradle.kts`
deliberately doesn't declare a toolchain that needs network access to
auto-provision one). CI (`.github/workflows/gradle-bridge.yml`) only runs
when files under this directory change — but note that `cargo build` at
the repo root *also* builds this project now (via `build.rs`, which runs
`./gradlew shadowJar` and embeds the result into the `jvmfast` binary), so
a JDK on `PATH` is required there too, not just here.

## Current state

Complete — both the server-side model builder and the client-side Tooling
API driver are implemented and tested against real Gradle builds/real
Maven Central (not just fixtures), and `jvmfast import-gradle` (Rust side,
`src/gradleimport/`) invokes this jar end to end. See CLAUDE.md's Fase 4
writeup for the full breakdown, and its "Known, deliberate gaps inside
Fase 4" for what's still out of scope (multi-project Gradle builds,
Gradle-side `java-version` extraction, BOM/exclusion/multi-repository
import).
