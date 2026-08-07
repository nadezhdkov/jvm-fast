package dev.jvmfast.gradlebridge.model;

import java.io.Serializable;
import java.util.List;

/**
 * The typed model jvmfast requests through the Gradle Tooling API
 * (docs/architecture.md seção 10) — one project's worth of dependency
 * data, resolved by Gradle itself and handed back through the Tooling
 * API's own binary protocol, never parsed out of {@code gradlew} stdout
 * text.
 *
 * <p>Populating a real implementation from {@code project.getConfigurations()}
 * is a separate, later milestone (see CLAUDE.md's Fase 4 gaps) — this
 * interface only fixes the shape both sides (the plugin running inside the
 * target build, and jvmfast's own driver on the client side) agree on.
 */
public interface JvmfastDependencyModel extends Serializable {
    List<JvmfastModule> getModules();
}
