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

That means a small piece of this feature has to be JVM code:

- A Gradle plugin (`JvmfastModelBuilderPlugin`) applied to the target
  project via a temporary init-script jvmfast generates, registering a
  `ToolingModelBuilder` (`JvmfastModelBuilder`) that runs inside the
  target build's own classloader.
- A typed model (`JvmfastDependencyModel`/`JvmfastModule`/
  `JvmfastDependency`) both that plugin and jvmfast's own driver code
  agree on the shape of.

## Building and testing

```shell
./gradlew build
```

Requires a JDK on `PATH` (any JDK 17+ works — `build.gradle.kts`
deliberately doesn't declare a toolchain that needs network access to
auto-provision one). CI (`.github/workflows/gradle-bridge.yml`) only runs
when files under this directory change.

## Current state

Only the plugin/model-builder registration skeleton exists —
`JvmfastModelBuilder.buildAll` throws `UnsupportedOperationException`
rather than resolving real dependencies, and nothing on the Rust side
(`jvmfast import-gradle`) invokes this jar yet. See CLAUDE.md's "Known,
deliberate gaps inside Fase 4" for the specifics.
