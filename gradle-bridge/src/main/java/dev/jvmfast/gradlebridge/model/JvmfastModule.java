package dev.jvmfast.gradlebridge.model;

import java.io.Serializable;
import java.util.List;

/**
 * One Gradle (sub)project's worth of resolved dependencies — a list, not a
 * single value, for the same reason `Workspace.modules` in the Rust domain
 * model is a `Vec<Module>` (docs/CONVENTIONS.md, Fase 5 compatibility
 * rules): multi-project Gradle builds are explicitly out of scope for
 * `import-gradle`'s first pass (docs/architecture.md seção 10), but the
 * model shape shouldn't need to change to add that support later.
 */
public interface JvmfastModule extends Serializable {
    String getName();

    /**
     * {@code project.getVersion()} as a string — Gradle's own default for
     * an unset project version ({@code "unspecified"}) is passed through
     * as-is; deciding what to substitute for it is the Rust-side driver's
     * job (it writes {@code project.toml}, this model only reports what
     * Gradle resolved).
     */
    String getVersion();

    List<JvmfastDependency> getDependencies();
}
