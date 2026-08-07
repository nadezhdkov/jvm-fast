# Style guide

_The following is a work-in-progress style guide for user-facing messaging in the CLI output and
documentation. Some of it — colored output, `--verbose`/`RUST_LOG` logging levels, the hints
system — describes target behavior ahead of the implementation, the same way
[`docs/architecture.md`](docs/architecture.md) is a living specification rather than a
description of what exists today. Check the current code (`src/cli/`) before assuming a described
behavior is already wired up._

## General

1. Use of "e.g." and "i.e." should always be wrapped in commas, e.g., as shown here.
1. Em-dashes are okay, but not recommended when using monospace fonts. Use "—", not "--" or "-".
1. Always wrap em-dashes in spaces, e.g., "hello — world" not "hello—world".
1. Hyphenate compound words, e.g., use "single-module" not "single module".
1. Use backticks to escape: commands, code expressions, package coordinates, and file paths.
1. Use less than and greater than symbols to wrap bare URLs, e.g., `<https://github.com/...>`
   (unless it is an example; then, use backticks).
1. Avoid bare URLs outside of reference documentation, prefer labels, e.g., `[name](url)`.
1. If a message ends with a single relevant value, precede it with a colon, e.g.,
   `This is the value: value`. If the value is a literal, wrap it in backticks.
1. Soft-wrap prose around 80–100 characters where it falls naturally, but do not hard-wrap to fit
   — `.markdownlint.yml` disables `MD013` (line length) on purpose, since this project's docs are
   long-form technical prose (`docs/architecture.md`, `CLAUDE.md`) where forcing a wrap column
   produces noisy diffs for no real benefit. Tables and code blocks are exempt.
1. Use a space, not an equals sign, for command-line arguments with a value, e.g.
   `--filter tag:fast`, not `--filter=tag:fast`.

## Styling jvm-fast

Just jvm-fast, please.

1. Do not escape with backticks, e.g., `jvm-fast`, unless referring specifically to the
   `jvmfast` executable.
1. Do not capitalize, e.g., "Jvm-fast", even at the beginning of a sentence.
1. Do not uppercase, e.g., "JVM-FAST", unless referring to an environment variable, e.g.,
   `JVMFAST_HOME`.

## Terminology

1. Use "lockfile" not "lock file" (the file itself is `project.lock`).
1. Use "manifest" to refer to `project.toml`, not "config file" or "project file".
1. Use "coordinate" (singular: `group:artifact`, or `group:artifact:version` when versioned) to
   refer to a Maven dependency identifier, not "package name".
1. Use "resolution" for the process that produces a `DependencyGraph`/`ResolvedNode` set, and
   "mediation" specifically for the conflict-winner step within it — do not use them
   interchangeably.
1. Use "JDK" not "jdk" or "Java SDK" in prose; the `jdk` subcommand itself is lowercase because
   it is a literal command name.
1. Use "pre-release" not "prerelease" (except in code, in which case: use `PreRelease` type
   names as established by the codebase, and `pre_release` field names).

## Documentation

1. Use periods at the end of all sentences, including lists unless they enumerate single items.
1. Avoid language that patronizes the reader, e.g., "simply do this".
1. Only refer to "the user" in internal or contributor documentation.
1. Avoid "we" in favor of "jvmfast" or imperative language, except in `docs/architecture.md`,
   which is written as design rationale and may use "we" when describing intent.

### Sections

The documentation is divided into:

1. Getting Started (README)
2. Architecture and conventions (`docs/architecture.md`, `docs/CONVENTIONS.md`)
3. Current status and roadmap (`CLAUDE.md`)

#### Getting Started

1. Should assume no previous knowledge about jvm-fast.
1. May assume basic knowledge of the Java/Maven ecosystem.
1. Should refer to `docs/architecture.md` for design rationale instead of repeating it.
1. Should have a clear flow: install → `install` → `build`/`run`/`test`.
1. Should not enumerate all CLI flags — that belongs in reference documentation once it exists.
1. Should be written from the second-person point of view.
1. Should use the imperative voice.

#### Architecture and conventions

1. Should cover design decisions and their rationale in detail.
1. Should be written from the third-person point of view, not second-person (i.e., avoid "you"),
   except where explicitly framed as design rationale ("we chose X because...").
1. Should not use the imperative voice.

#### Status and roadmap

1. Should enumerate milestones and their current state precisely (done / in progress / not
   started), matching the checklist style used in the README's "Status do Projeto" section.
1. Should be written from the third-person point of view.

### Code blocks

1. All code blocks should have a language marker.
1. When using `console` syntax, use `$` to indicate commands — everything else is output.
1. Never use the `bash` syntax when displaying command output.
1. Prefer `console` with `$`-prefixed commands over `bash`.
1. Command output should rarely be included — it's hard to keep up-to-date.
1. Use `title` for example files, e.g., `project.toml`, `project.lock`, or `Main.java`.

## CLI

1. Do not use periods at the end of sentences :), unless the message spans more than a single
   sentence.
1. May use the second-person point of view, e.g., "Did you mean...?".

### Colors and style

1. All CLI output must be interpretable and understandable _without_ the use of color and other
   styling. (For example: even if a command is rendered in green, wrap it in backticks.)
1. `NO_COLOR` must be respected when using any colors or styling.
1. In general, use:
   - Green for success.
   - Red for error.
   - Yellow for warning.
   - Cyan for hints.
   - Cyan for file paths.
   - Cyan for important user-facing literals (e.g., a coordinate in a message).
   - Green for commands.

### Logging

1. `warn`, `info`, `debug`, and `trace` logs are all shown with the `--verbose` flag.
   - Note that the displayed level is controlled with `RUST_LOG`.
1. All logging should be to stderr.

### Output

1. Text can be written to stdout if it is "data" that could be piped to another program (e.g.,
   `jvmfast tree`).

### Warnings

1. `warn_user` and `warn_user_once` should be preferred over tracing warnings when the warning is
   actionable, e.g., a manifest field that will change behavior in a future version.
1. Deprecation warnings must be actionable.

### Hints

1. Errors may be followed by hints suggesting a solution, e.g., a missing `[run].main-class`
   pointing at the manifest section to add.
1. Hints should be separated from errors by a blank newline.
1. Hints should be stylized as `hint: <content>`.
