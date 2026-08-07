package dev.jvmfast.gradlebridge.model;

import java.io.Serializable;

/**
 * One dependency the way Gradle itself resolved it, for one configuration
 * (e.g. {@code compileClasspath}, {@code runtimeClasspath},
 * {@code testCompileClasspath}). Deciding how a configuration maps onto
 * jvm-fast's own {@code [dependencies]}/{@code [dev-dependencies]} split
 * is the Rust-side driver's job (docs/architecture.md seção 10), not
 * this model's — it only reports what Gradle resolved, without
 * interpreting it.
 */
public interface JvmfastDependency extends Serializable {
    String getCoordinate();

    String getVersion();

    String getConfiguration();
}
