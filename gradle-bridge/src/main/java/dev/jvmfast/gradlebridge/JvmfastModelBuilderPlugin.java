package dev.jvmfast.gradlebridge;

import org.gradle.api.Plugin;
import org.gradle.api.Project;
import org.gradle.tooling.provider.model.ToolingModelBuilderRegistry;

import javax.inject.Inject;

/**
 * Applied to the target project via the init-script jvmfast generates at
 * import time (docs/architecture.md seção 10, step 1: "aplica um plugin
 * registrando um ToolingModelBuilder customizado") — registers
 * {@link JvmfastModelBuilder} so the Tooling API model request jvmfast's
 * driver makes (step 3, not implemented yet on the Rust side — see
 * CLAUDE.md) resolves to it. Gradle injects {@link ToolingModelBuilderRegistry}
 * itself; this class never looks it up from {@code project}.
 */
public class JvmfastModelBuilderPlugin implements Plugin<Project> {

    private final ToolingModelBuilderRegistry registry;

    @Inject
    public JvmfastModelBuilderPlugin(ToolingModelBuilderRegistry registry) {
        this.registry = registry;
    }

    @Override
    public void apply(Project project) {
        registry.register(new JvmfastModelBuilder());
    }
}
